use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::FileStorage;
use crate::domain::entities::File;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};
use crate::domain::value_objects::{FileHash, FileMetadata};

#[derive(Debug)]
pub enum UploadFileError {
    StorageError(String),
    RepositoryError(String),
    ValidationError(String),
    DuplicateFile(String),
}

impl std::fmt::Display for UploadFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadFileError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            UploadFileError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            UploadFileError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            UploadFileError::DuplicateFile(msg) => write!(f, "Duplicate file: {}", msg),
        }
    }
}

impl std::error::Error for UploadFileError {}

impl From<FileRepositoryError> for UploadFileError {
    fn from(error: FileRepositoryError) -> Self {
        UploadFileError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct UploadFileRequest {
    pub file_name: String,
    pub file_data: Vec<u8>,
    pub content_type: Option<String>,
    pub metadata: Option<FileMetadata>,
}

#[derive(Debug, Clone)]
pub struct UploadFileResponse {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub content_type: Option<String>,
}

pub struct UploadFileUseCase {
    file_repository: Arc<dyn FileRepository>,
    file_storage: Arc<dyn FileStorage>,
}

impl UploadFileUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        file_storage: Arc<dyn FileStorage>,
    ) -> Self {
        Self {
            file_repository,
            file_storage,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: UploadFileRequest,
    ) -> Result<UploadFileResponse, UploadFileError> {
        // Validate input
        if request.file_name.trim().is_empty() {
            return Err(UploadFileError::ValidationError(
                "File name cannot be empty".to_string(),
            ));
        }

        if request.file_data.is_empty() {
            return Err(UploadFileError::ValidationError(
                "File data cannot be empty".to_string(),
            ));
        }

        // Generate file hash
        let file_hash = FileHash::from_bytes(&request.file_data);

        // Check for duplicate files
        if let Ok(Some(_)) = self
            .file_repository
            .find_by_hash(tenant_id, file_hash.as_str())
            .await
        {
            return Err(UploadFileError::DuplicateFile(
                "File with this hash already exists".to_string(),
            ));
        }

        let file_id = Uuid::new_v4();

        // Store file
        let stored_file = self
            .file_storage
            .store_file(
                tenant_id,
                file_id,
                &request.file_data,
                &request.file_name,
                request.content_type.as_deref(),
            )
            .await
            .map_err(|e| UploadFileError::StorageError(e.to_string()))?;

        // Create domain entity
        let file = File::new(
            stored_file.key,
            request.file_name.clone(),
            Some(request.file_data.len() as i64),
            request.content_type.clone(),
            Some(file_hash.clone()),
            request.metadata,
        );

        // Save to repository and verify the generated id round-trips
        let saved_id = self.file_repository.save(tenant_id, &file).await?;
        if saved_id != file_id {
            let _ = self.file_storage.delete(tenant_id, file_id).await;
            return Err(UploadFileError::RepositoryError(format!(
                "File id mismatch after save: storage={} db={}",
                file_id, saved_id
            )));
        }

        Ok(UploadFileResponse {
            file_id,
            file_name: request.file_name,
            file_size: request.file_data.len() as i64,
            file_hash: file_hash.to_string(),
            content_type: request.content_type,
        })
    }
}
