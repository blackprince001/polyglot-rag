use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{JobQueue, job_queue::JobQueueError};
use crate::domain::repositories::{JobRepository, job_repository::JobRepositoryError};

#[derive(Debug)]
pub enum CancelJobError {
    JobNotFound(Uuid),
    RepositoryError(String),
    QueueError(String),
    JobNotCancellable(String),
}

impl std::fmt::Display for CancelJobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CancelJobError::JobNotFound(id) => write!(f, "Job not found: {}", id),
            CancelJobError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            CancelJobError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            CancelJobError::JobNotCancellable(msg) => write!(f, "Job not cancellable: {}", msg),
        }
    }
}

impl std::error::Error for CancelJobError {}

impl From<JobRepositoryError> for CancelJobError {
    fn from(error: JobRepositoryError) -> Self {
        CancelJobError::RepositoryError(error.to_string())
    }
}

impl From<JobQueueError> for CancelJobError {
    fn from(error: JobQueueError) -> Self {
        CancelJobError::QueueError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct CancelJobRequest {
    pub job_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct CancelJobResponse {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

pub struct CancelJobUseCase {
    job_repository: Arc<dyn JobRepository>,
    job_queue: Arc<dyn JobQueue>,
}

impl CancelJobUseCase {
    pub fn new(job_repository: Arc<dyn JobRepository>, job_queue: Arc<dyn JobQueue>) -> Self {
        Self {
            job_repository,
            job_queue,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: CancelJobRequest,
    ) -> Result<CancelJobResponse, CancelJobError> {
        // Find the job
        let mut job = self
            .job_repository
            .find_by_id(tenant_id, request.job_id)
            .await?
            .ok_or(CancelJobError::JobNotFound(request.job_id))?;

        // Check if job can be cancelled
        if !job.is_active() {
            return Err(CancelJobError::JobNotCancellable(format!(
                "Job is in {:?} state and cannot be cancelled",
                job.status()
            )));
        }

        // Try to remove from queue if it's still pending
        if job.status().is_pending() {
            let _ = self.job_queue.remove_job(request.job_id).await; // Don't fail if not in queue
        }

        // Cancel the job
        job.cancel()
            .map_err(|e| CancelJobError::JobNotCancellable(e))?;

        // Update in repository
        self.job_repository.update(&job).await?;

        Ok(CancelJobResponse {
            job_id: request.job_id,
            status: "cancelled".to_string(),
            message: "Job cancelled successfully".to_string(),
        })
    }
}
