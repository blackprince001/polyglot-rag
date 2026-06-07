use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::File;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};

#[derive(Debug)]
pub enum GetFileError {
    FileNotFound(Uuid),
    RepositoryError(String),
}

impl std::fmt::Display for GetFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetFileError::FileNotFound(id) => write!(f, "File not found: {}", id),
            GetFileError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for GetFileError {}

impl From<FileRepositoryError> for GetFileError {
    fn from(error: FileRepositoryError) -> Self {
        GetFileError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct GetFileRequest {
    pub file_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetFileResponse {
    pub file: File,
}

pub struct GetFileUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl GetFileUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: GetFileRequest,
    ) -> Result<GetFileResponse, GetFileError> {
        let file = self
            .file_repository
            .find_by_id(tenant_id, request.file_id)
            .await?
            .ok_or(GetFileError::FileNotFound(request.file_id))?;

        Ok(GetFileResponse { file })
    }
}
