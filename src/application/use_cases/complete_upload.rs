use std::sync::Arc;
use uuid::Uuid;

use crate::application::use_cases::queue_processing_job::{
    QueueJobError, QueueJobRequest, QueueProcessingJobUseCase,
};
use crate::domain::entities::processing_job::JobType;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};

#[derive(Debug)]
pub enum CompleteUploadError {
    FileNotFound(Uuid),
    RepositoryError(String),
    QueueError(String),
}

impl std::fmt::Display for CompleteUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompleteUploadError::FileNotFound(id) => write!(f, "File {} not found", id),
            CompleteUploadError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            CompleteUploadError::QueueError(msg) => write!(f, "Queue error: {}", msg),
        }
    }
}

impl std::error::Error for CompleteUploadError {}

impl From<FileRepositoryError> for CompleteUploadError {
    fn from(error: FileRepositoryError) -> Self {
        CompleteUploadError::RepositoryError(error.to_string())
    }
}

impl From<QueueJobError> for CompleteUploadError {
    fn from(error: QueueJobError) -> Self {
        CompleteUploadError::QueueError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct CompleteUploadRequest {
    pub file_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CompleteUploadResponse {
    pub file_id: Uuid,
    pub job_id: Uuid,
    pub status: String,
}

pub struct CompleteUploadUseCase {
    file_repository: Arc<dyn FileRepository>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
}

impl CompleteUploadUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        queue_job_use_case: Arc<QueueProcessingJobUseCase>,
    ) -> Self {
        Self {
            file_repository,
            queue_job_use_case,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: CompleteUploadRequest,
    ) -> Result<CompleteUploadResponse, CompleteUploadError> {
        let _ = self
            .file_repository
            .find_by_id(tenant_id, request.file_id)
            .await?
            .ok_or(CompleteUploadError::FileNotFound(request.file_id))?;

        let queue_request = QueueJobRequest {
            file_id: request.file_id,
            job_type: JobType::FileProcessing,
        };
        let queue_response = self
            .queue_job_use_case
            .execute(tenant_id, queue_request)
            .await?;

        Ok(CompleteUploadResponse {
            file_id: request.file_id,
            job_id: queue_response.job_id,
            status: queue_response.status,
        })
    }
}
