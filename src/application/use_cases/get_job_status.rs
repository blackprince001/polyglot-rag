use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::ProcessingJob;
use crate::domain::repositories::{JobRepository, job_repository::JobRepositoryError};

#[derive(Debug)]
pub enum GetJobStatusError {
    JobNotFound(Uuid),
    RepositoryError(String),
}

impl std::fmt::Display for GetJobStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetJobStatusError::JobNotFound(id) => write!(f, "Job not found: {}", id),
            GetJobStatusError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for GetJobStatusError {}

impl From<JobRepositoryError> for GetJobStatusError {
    fn from(error: JobRepositoryError) -> Self {
        GetJobStatusError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct GetJobStatusRequest {
    pub job_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct GetJobStatusResponse {
    pub job: ProcessingJob,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
    pub duration: Option<chrono::Duration>,
}

pub struct GetJobStatusUseCase {
    job_repository: Arc<dyn JobRepository>,
}

impl GetJobStatusUseCase {
    pub fn new(job_repository: Arc<dyn JobRepository>) -> Self {
        Self { job_repository }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: GetJobStatusRequest,
    ) -> Result<GetJobStatusResponse, GetJobStatusError> {
        let job = self
            .job_repository
            .find_by_id(tenant_id, request.job_id)
            .await?
            .ok_or(GetJobStatusError::JobNotFound(request.job_id))?;

        Ok(GetJobStatusResponse {
            estimated_completion: job.estimated_completion(),
            duration: job.duration(),
            job,
        })
    }

    pub async fn get_jobs_for_file(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<ProcessingJob>, GetJobStatusError> {
        self.job_repository
            .find_by_file_id(tenant_id, file_id)
            .await
            .map_err(GetJobStatusError::from)
    }

    pub async fn get_active_jobs(&self) -> Result<Vec<ProcessingJob>, GetJobStatusError> {
        self.job_repository
            .find_active_jobs()
            .await
            .map_err(GetJobStatusError::from)
    }
}
