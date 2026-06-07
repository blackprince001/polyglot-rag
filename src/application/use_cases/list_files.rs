use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::File;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};

#[derive(Debug)]
pub enum ListFilesError {
    RepositoryError(String),
    ValidationError(String),
}

impl std::fmt::Display for ListFilesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListFilesError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            ListFilesError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ListFilesError {}

impl From<FileRepositoryError> for ListFilesError {
    fn from(error: FileRepositoryError) -> Self {
        ListFilesError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct ListFilesRequest {
    pub skip: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct ListFilesResponse {
    pub files: Vec<File>,
    pub total_count: i64,
    pub skip: i64,
    pub limit: i64,
}

pub struct ListFilesUseCase {
    file_repository: Arc<dyn FileRepository>,
}

impl ListFilesUseCase {
    pub fn new(file_repository: Arc<dyn FileRepository>) -> Self {
        Self { file_repository }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: ListFilesRequest,
    ) -> Result<ListFilesResponse, ListFilesError> {
        // Validate input
        if request.skip < 0 {
            return Err(ListFilesError::ValidationError(
                "Skip cannot be negative".to_string(),
            ));
        }

        if request.limit <= 0 || request.limit > 1000 {
            return Err(ListFilesError::ValidationError(
                "Limit must be between 1 and 1000".to_string(),
            ));
        }

        // Get files and total count
        let files = self
            .file_repository
            .find_all(tenant_id, request.skip, request.limit)
            .await?;
        let total_count = self.file_repository.count(tenant_id).await?;

        Ok(ListFilesResponse {
            files,
            total_count,
            skip: request.skip,
            limit: request.limit,
        })
    }
}
