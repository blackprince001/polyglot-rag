use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{
    FileStorage,
    file_storage::{PresignedUpload, storage_key},
};
use crate::domain::entities::File;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};
use crate::domain::value_objects::FileMetadata;

#[derive(Debug)]
pub enum RequestUploadUrlError {
    ValidationError(String),
    StorageError(String),
    RepositoryError(String),
}

impl std::fmt::Display for RequestUploadUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestUploadUrlError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            RequestUploadUrlError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            RequestUploadUrlError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for RequestUploadUrlError {}

impl From<FileRepositoryError> for RequestUploadUrlError {
    fn from(error: FileRepositoryError) -> Self {
        RequestUploadUrlError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct RequestUploadUrlRequest {
    pub file_name: String,
    pub content_type: Option<String>,

    pub expiry_secs: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RequestUploadUrlResponse {
    pub file_id: Uuid,
    pub file_name: String,
    pub method: String,

    pub url: Option<String>,
    pub headers: Vec<(String, String)>,
    pub form_fields: Vec<(String, String)>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub struct RequestUploadUrlUseCase {
    file_repository: Arc<dyn FileRepository>,
    file_storage: Arc<dyn FileStorage>,
    presigned_upload_ttl_secs: u64,
}

impl RequestUploadUrlUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        file_storage: Arc<dyn FileStorage>,
        presigned_upload_ttl_secs: u64,
    ) -> Self {
        Self {
            file_repository,
            file_storage,
            presigned_upload_ttl_secs,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: RequestUploadUrlRequest,
    ) -> Result<RequestUploadUrlResponse, RequestUploadUrlError> {
        let file_name = request.file_name.trim().to_string();
        if file_name.is_empty() {
            return Err(RequestUploadUrlError::ValidationError(
                "file_name cannot be empty".to_string(),
            ));
        }
        if file_name.contains('/') || file_name.contains("..") {
            return Err(RequestUploadUrlError::ValidationError(
                "file_name must not contain '/' or '..'".to_string(),
            ));
        }

        let file_id = Uuid::new_v4();
        let expiry = std::time::Duration::from_secs(
            request
                .expiry_secs
                .unwrap_or(self.presigned_upload_ttl_secs),
        );

        let presigned: PresignedUpload = self
            .file_storage
            .presigned_upload_url(
                tenant_id,
                file_id,
                &file_name,
                request.content_type.as_deref(),
                expiry,
            )
            .await
            .map_err(|e| RequestUploadUrlError::StorageError(e.to_string()))?;

        let file = File::new(
            storage_key(tenant_id, file_id),
            file_name.clone(),
            None,
            request.content_type.clone(),
            None,
            Some(FileMetadata::new()),
        );
        let saved_id = self.file_repository.save(tenant_id, &file).await?;
        if saved_id != file_id {
            return Err(RequestUploadUrlError::RepositoryError(format!(
                "File id mismatch after save: storage={} db={}",
                file_id, saved_id
            )));
        }

        Ok(RequestUploadUrlResponse {
            file_id,
            file_name,
            method: presigned.method,
            url: presigned.url,
            headers: presigned.headers,
            form_fields: presigned.form_fields,
            expires_at: presigned.expires_at,
        })
    }
}
