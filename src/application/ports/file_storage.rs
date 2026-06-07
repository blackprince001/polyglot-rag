use async_trait::async_trait;
use std::time::Duration;
use tokio::io::AsyncRead;
use uuid::Uuid;

#[derive(Debug)]
pub enum FileStorageError {
    IoError(String),
    Backend(String),
    Unsupported,
    PresignFailed(String),
}

impl std::fmt::Display for FileStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileStorageError::IoError(msg) => write!(f, "IO error: {}", msg),
            FileStorageError::Backend(msg) => write!(f, "Storage backend error: {}", msg),
            FileStorageError::Unsupported => {
                write!(f, "Operation not supported by this storage backend")
            }
            FileStorageError::PresignFailed(msg) => write!(f, "Presign failed: {}", msg),
        }
    }
}

impl std::error::Error for FileStorageError {}

#[derive(Debug, Clone)]
pub struct StoredFile {
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct PresignedUpload {
    pub url: Option<String>,
    pub method: String,
    pub headers: Vec<(String, String)>,
    pub form_fields: Vec<(String, String)>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct PresignedDownload {
    pub url: String,
}

pub fn storage_key(tenant_id: Uuid, file_id: Uuid) -> String {
    format!("{}/{}", tenant_id, file_id)
}

#[async_trait]
pub trait FileStorage: Send + Sync {
    async fn store_file(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        data: &[u8],
        file_name: &str,
        content_type: Option<&str>,
    ) -> Result<StoredFile, FileStorageError>;

    async fn presigned_upload_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        file_name: &str,
        content_type: Option<&str>,
        expiry: Duration,
    ) -> Result<PresignedUpload, FileStorageError>;

    async fn presigned_download_url(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        expiry: Duration,
    ) -> Result<PresignedDownload, FileStorageError>;

    async fn open_read(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Box<dyn AsyncRead + Send + Unpin>, FileStorageError>;

    async fn delete(&self, tenant_id: Uuid, file_id: Uuid) -> Result<(), FileStorageError>;

    fn supports_server_stream(&self) -> bool;
}
