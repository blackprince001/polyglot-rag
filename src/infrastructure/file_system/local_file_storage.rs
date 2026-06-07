use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncRead;
use uuid::Uuid;

use crate::application::ports::file_storage::{
    FileStorage, FileStorageError, PresignedDownload, PresignedUpload, StoredFile,
};

pub struct LocalFileStorage {
    base_path: PathBuf,
}

impl LocalFileStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub async fn ensure_directory_exists(&self) -> Result<(), FileStorageError> {
        fs::create_dir_all(&self.base_path)
            .await
            .map_err(|e| FileStorageError::IoError(e.to_string()))
    }

    pub fn key(tenant_id: Uuid, file_id: Uuid) -> String {
        format!("{}/{}", tenant_id, file_id)
    }

    fn path_for(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl FileStorage for LocalFileStorage {
    async fn store_file(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        data: &[u8],
        _file_name: &str,
        _content_type: Option<&str>,
    ) -> Result<StoredFile, FileStorageError> {
        self.ensure_directory_exists().await?;
        let key = Self::key(tenant_id, file_id);
        let path = self.path_for(&key);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| FileStorageError::IoError(e.to_string()))?;
        }

        fs::write(&path, data)
            .await
            .map_err(|e| FileStorageError::IoError(e.to_string()))?;

        Ok(StoredFile { key })
    }

    async fn presigned_upload_url(
        &self,
        _tenant_id: Uuid,
        _file_id: Uuid,
        _file_name: &str,
        _content_type: Option<&str>,
        _expiry: Duration,
    ) -> Result<PresignedUpload, FileStorageError> {
        Ok(PresignedUpload {
            url: None,
            method: "POST".to_string(),
            headers: Vec::new(),
            form_fields: Vec::new(),
            expires_at: chrono::Utc::now(),
        })
    }

    async fn presigned_download_url(
        &self,
        _tenant_id: Uuid,
        file_id: Uuid,
        _expiry: Duration,
    ) -> Result<PresignedDownload, FileStorageError> {
        Ok(PresignedDownload {
            url: format!("/files/{}/content", file_id),
        })
    }

    async fn open_read(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        let path = self.path_for(&key);
        match fs::File::open(&path).await {
            Ok(file) => Ok(Box::new(file)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(FileStorageError::IoError(
                format!("object not found at key '{}'", key),
            )),
            Err(e) => Err(FileStorageError::IoError(e.to_string())),
        }
    }

    async fn delete(&self, tenant_id: Uuid, file_id: Uuid) -> Result<(), FileStorageError> {
        let key = Self::key(tenant_id, file_id);
        let path = self.path_for(&key);
        match fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(FileStorageError::IoError(e.to_string())),
        }
    }

    fn supports_server_stream(&self) -> bool {
        true
    }
}
