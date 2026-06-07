use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::ProcessingJob;

#[derive(Debug)]
pub enum JobQueueError {
    ConnectionError(String),
}

impl std::fmt::Display for JobQueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobQueueError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
        }
    }
}

impl std::error::Error for JobQueueError {}

#[async_trait]
pub trait JobQueue: Send + Sync {
    /// Enqueue a job for processing.
    async fn enqueue(&self, job: ProcessingJob) -> Result<(), JobQueueError>;

    /// Remove a specific job from the queue (for cancellation).
    async fn remove_job(&self, job_id: Uuid) -> Result<bool, JobQueueError>;
}
