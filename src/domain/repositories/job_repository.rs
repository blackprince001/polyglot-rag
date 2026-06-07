use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::ProcessingJob;

#[derive(Debug)]
pub enum JobRepositoryError {
    DatabaseError(String),
}

impl std::fmt::Display for JobRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for JobRepositoryError {}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
    ) -> Result<Option<ProcessingJob>, JobRepositoryError>;
    async fn find_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<ProcessingJob>, JobRepositoryError>;
    async fn find_active_jobs(&self) -> Result<Vec<ProcessingJob>, JobRepositoryError>;
    async fn update(&self, job: &ProcessingJob) -> Result<(), JobRepositoryError>;
}
