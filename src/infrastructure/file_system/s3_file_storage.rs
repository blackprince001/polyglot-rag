use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;
use tokio::io::AsyncRead;
use uuid::Uuid;

use crate::application::ports::file_storage::{
    FileStorage, FileStorageError, PresignedDownload, PresignedUpload, StoredFile,
};
use crate::infrastructure::file_system::s3_config::S3Config;

pub struct S3FileStorage {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3FileStorage {
    pub async fn new(config: S3Config) -> Result<Self, FileStorageError> {
        let creds = Credentials::new(
            config.access_key_id,
            config.secret_access_key,
            None,
            None,
            "polyrag-static",
        );
        let mut builder = aws_sdk_s3::Config::builder()
            .region(Region::new(config.region.clone()))
            .credentials_provider(creds)
            .force_path_style(config.force_path_style);
        if let Some(endpoint) = &config.endpoint {
            builder = builder.endpoint_url(endpoint);
        }
        let s3_config = builder.build();
        let client = aws_sdk_s3::Client::from_conf(s3_config);
        Ok(Self {
            client,
            bucket: config.bucket,
        })
    }

    /// Same key shape as the local backend: `"{tenant_id}/{file_id}"`.
    pub fn key(tenant_id: Uuid, file_id: Uuid) -> String {
        format!("{}/{}", tenant_id, file_id)
    }

    fn to_sdk_expiry(expiry: Duration) -> Result<PresigningConfig, FileStorageError> {
        // S3 presigning requires a positive expiry between 1s and 7 days.
        PresigningConfig::expires_in(expiry)
            .map_err(|e| FileStorageError::PresignFailed(e.to_string()))
    }
}

#[async_trait]
impl FileStorage for S3FileStorage {
    async fn store_file(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        data: &[u8],
        _file_name: &str,
        content_type: Option<&str>,
    ) -> Result<StoredFile, FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        let mut req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(data.to_vec().into());
        if let Some(ct) = content_type {
            req = req.content_type(ct);
        }
        req.send()
            .await
            .map_err(|e| FileStorageError::Backend(format!("S3 put_object: {}", e)))?;
        Ok(StoredFile { key })
    }

    async fn presigned_upload_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        _file_name: &str,
        content_type: Option<&str>,
        expiry: Duration,
    ) -> Result<PresignedUpload, FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        let presigning = Self::to_sdk_expiry(expiry)?;
        let content_type_owned = content_type
            .unwrap_or("application/octet-stream")
            .to_string();
        let presigned = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(content_type_owned.clone())
            .presigned(presigning)
            .await
            .map_err(|e| FileStorageError::PresignFailed(format!("S3 put presign: {}", e)))?;
        Ok(PresignedUpload {
            url: Some(presigned.uri().to_string()),
            method: "PUT".to_string(),
            // S3 signs the URL with the `content_type` we set, so the client
            // must send the matching header verbatim.
            headers: vec![("Content-Type".to_string(), content_type_owned)],
            form_fields: Vec::new(),
            expires_at: chrono::Utc::now()
                + chrono::Duration::from_std(expiry).unwrap_or(chrono::Duration::seconds(900)),
        })
    }

    async fn presigned_download_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        expiry: Duration,
    ) -> Result<PresignedDownload, FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        let presigning = Self::to_sdk_expiry(expiry)?;
        let presigned = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .presigned(presigning)
            .await
            .map_err(|e| FileStorageError::PresignFailed(format!("S3 get presign: {}", e)))?;
        Ok(PresignedDownload {
            url: presigned.uri().to_string(),
        })
    }

    async fn open_read(
        &self,
        _tenant_id: Uuid,
        _file_id: Uuid,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, FileStorageError> {
        // S3 never proxies bytes through the server — callers use the
        // presigned GET URL.
        Err(FileStorageError::Unsupported)
    }

    async fn delete(&self, tenant_id: Uuid, file_id: Uuid) -> Result<(), FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        match self
            .client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("NoSuchKey") || msg.contains("404") {
                    Ok(())
                } else {
                    Err(FileStorageError::Backend(format!(
                        "S3 delete_object: {}",
                        msg
                    )))
                }
            }
        }
    }

    fn supports_server_stream(&self) -> bool {
        false
    }
}
