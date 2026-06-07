use async_trait::async_trait;
use chrono::Utc;
use sha1::{Digest, Sha1};
use std::time::Duration;
use tokio::io::AsyncRead;
use uuid::Uuid;

use crate::application::ports::file_storage::{
    FileStorage, FileStorageError, PresignedDownload, PresignedUpload, StoredFile,
};
use crate::infrastructure::file_system::cloudinary_config::CloudinaryConfig;

pub struct CloudinaryFileStorage {
    config: CloudinaryConfig,
    upload_client: reqwest::Client,
    control_client: reqwest::Client,
}

impl CloudinaryFileStorage {
    pub fn new(config: CloudinaryConfig) -> Result<Self, FileStorageError> {
        let upload_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|e| FileStorageError::Backend(format!("build upload client: {}", e)))?;
        let control_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| FileStorageError::Backend(format!("build control client: {}", e)))?;
        Ok(Self {
            config,
            upload_client,
            control_client,
        })
    }

    fn upload_url(&self) -> String {
        format!(
            "https://api.cloudinary.com/v1_1/{}/image/upload",
            self.config.cloud_name
        )
    }

    fn destroy_url(&self) -> String {
        format!(
            "https://api.cloudinary.com/v1_1/{}/image/destroy",
            self.config.cloud_name
        )
    }

    fn cdn_url(&self, public_id: &str) -> String {
        format!(
            "https://res.cloudinary.com/{}/image/upload/{}",
            self.config.cloud_name, public_id
        )
    }

    fn sign_params(params: &[(&str, String)], api_secret: &str) -> String {
        let mut sorted: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        let body: String = sorted
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let to_hash = format!("{}{}", body, api_secret);
        let mut hasher = Sha1::new();
        hasher.update(to_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[async_trait]
impl FileStorage for CloudinaryFileStorage {
    async fn store_file(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        data: &[u8],
        file_name: &str,
        content_type: Option<&str>,
    ) -> Result<StoredFile, FileStorageError> {
        let public_id = self.config.public_id(tenant_id, file_id);
        let mut form = reqwest::multipart::Form::new()
            .text("public_id", public_id.clone())
            .text("resource_type", "image")
            .text("overwrite", "true")
            .part(
                "file",
                reqwest::multipart::Part::bytes(data.to_vec())
                    .file_name(file_name.to_string())
                    .mime_str(content_type.unwrap_or("application/octet-stream"))
                    .map_err(|e| {
                        FileStorageError::Backend(format!("invalid content-type: {}", e))
                    })?,
            );
        if let Some(folder) = &self.config.folder {
            form = form.text("folder", folder.clone());
        }

        let response = self
            .upload_client
            .post(self.upload_url())
            .header("Authorization", self.config.basic_auth_header())
            .multipart(form)
            .send()
            .await
            .map_err(|e| FileStorageError::Backend(format!("Cloudinary upload send: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(FileStorageError::Backend(format!(
                "Cloudinary upload returned {}: {}",
                status, body
            )));
        }
        Ok(StoredFile { key: public_id })
    }

    async fn presigned_upload_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        _file_name: &str,
        _content_type: Option<&str>,
        expiry: Duration,
    ) -> Result<PresignedUpload, FileStorageError> {
        let public_id = self.config.public_id(tenant_id, file_id);
        let timestamp = Utc::now().timestamp().to_string();

        let mut params: Vec<(&str, String)> = vec![
            ("overwrite", "true".to_string()),
            ("public_id", public_id.clone()),
            ("timestamp", timestamp.clone()),
        ];
        if let Some(folder) = &self.config.folder {
            params.push(("folder", folder.clone()));
        }
        let signature = Self::sign_params(&params, &self.config.api_secret);

        let mut form_fields = vec![
            ("api_key".to_string(), self.config.api_key.clone()),
            ("timestamp".to_string(), timestamp),
            ("signature".to_string(), signature),
            ("public_id".to_string(), public_id),
            ("overwrite".to_string(), "true".to_string()),
        ];
        if let Some(folder) = &self.config.folder {
            form_fields.push(("folder".to_string(), folder.clone()));
        }

        let expires_at = Utc::now()
            + chrono::Duration::from_std(expiry).unwrap_or(chrono::Duration::seconds(900));
        Ok(PresignedUpload {
            url: Some(self.upload_url()),
            method: "POST".to_string(),
            headers: Vec::new(),
            form_fields,
            expires_at,
        })
    }

    async fn presigned_download_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        _expiry: Duration,
    ) -> Result<PresignedDownload, FileStorageError> {
        let public_id = self.config.public_id(tenant_id, file_id);
        // Public delivery URL. Security model relies on the UUID being
        // unguessable + the server-side auth gate on presign minting.
        Ok(PresignedDownload {
            url: self.cdn_url(&public_id),
        })
    }

    async fn open_read(
        &self,
        _tenant_id: Uuid,
        _file_id: Uuid,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, FileStorageError> {
        // Cloudinary delivers via CDN — clients hit the presigned URL.
        Err(FileStorageError::Unsupported)
    }

    async fn delete(&self, tenant_id: Uuid, file_id: Uuid) -> Result<(), FileStorageError> {
        let public_id = self.config.public_id(tenant_id, file_id);
        let response = self
            .control_client
            .post(self.destroy_url())
            .header("Authorization", self.config.basic_auth_header())
            .form(&[("public_id", public_id.as_str()), ("type", "upload")])
            .send()
            .await
            .map_err(|e| FileStorageError::Backend(format!("Cloudinary destroy send: {}", e)))?;

        let status = response.status();
        if status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if body.contains("\"not found\"") {
                return Ok(());
            }
            return Ok(());
        }
        if status.as_u16() == 404 {
            return Ok(());
        }
        let body = response.text().await.unwrap_or_default();
        Err(FileStorageError::Backend(format!(
            "Cloudinary destroy returned {}: {}",
            status, body
        )))
    }

    fn supports_server_stream(&self) -> bool {
        false
    }
}
