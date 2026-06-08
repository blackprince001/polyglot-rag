use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{JobQueue, job_queue::JobQueueError};
use crate::domain::entities::{ProcessingJob, processing_job::JobType};
use crate::domain::repositories::{
    FileRepository, JobRepository, job_repository::JobRepositoryError,
};

#[derive(Debug)]
pub enum QueueJobError {
    FileNotFound(Uuid),
    RepositoryError(String),
    QueueError(String),
    ValidationError(String),
}

impl std::fmt::Display for QueueJobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueJobError::FileNotFound(id) => write!(f, "File not found: {}", id),
            QueueJobError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            QueueJobError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            QueueJobError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for QueueJobError {}

impl From<JobRepositoryError> for QueueJobError {
    fn from(error: JobRepositoryError) -> Self {
        QueueJobError::RepositoryError(error.to_string())
    }
}

impl From<JobQueueError> for QueueJobError {
    fn from(error: JobQueueError) -> Self {
        QueueJobError::QueueError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct QueueJobRequest {
    pub file_id: Uuid,
    pub job_type: JobType,
}

#[derive(Debug, Clone)]
pub struct QueueJobResponse {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub job_type: JobType,
    pub status: String,
    pub message: String,
}

pub struct QueueProcessingJobUseCase {
    job_repository: Arc<dyn JobRepository>,
    job_queue: Arc<dyn JobQueue>,
    file_repository: Arc<dyn FileRepository>,
}

impl QueueProcessingJobUseCase {
    pub fn new(
        job_repository: Arc<dyn JobRepository>,
        job_queue: Arc<dyn JobQueue>,
        file_repository: Arc<dyn FileRepository>,
    ) -> Self {
        Self {
            job_repository,
            job_queue,
            file_repository,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: QueueJobRequest,
    ) -> Result<QueueJobResponse, QueueJobError> {
        // Validate that the file exists
        let file = self
            .file_repository
            .find_by_id(tenant_id, request.file_id)
            .await
            .map_err(|e| QueueJobError::RepositoryError(e.to_string()))?
            .ok_or_else(|| QueueJobError::FileNotFound(request.file_id))?;

        // Check if there's already an active job for this file
        let existing_jobs = self
            .job_repository
            .find_by_file_id(tenant_id, file.id())
            .await?;
        if existing_jobs.iter().any(|job| job.is_active()) {
            return Err(QueueJobError::ValidationError(
                "File already has an active processing job".to_string(),
            ));
        }

        // Create the processing job based on type
        let job = match &request.job_type {
            JobType::FileProcessing => {
                ProcessingJob::new_file_processing(tenant_id, request.file_id)
            }
            JobType::UrlExtraction { url } => {
                ProcessingJob::new_url_extraction(tenant_id, request.file_id, url.clone())
            }
            JobType::YoutubeExtraction { url } => {
                ProcessingJob::new_youtube_extraction(tenant_id, request.file_id, url.clone())
            }
        };

        let job_id = job.id();

        self.job_repository.save(&job).await?;

        self.job_queue.enqueue(job).await?;

        Ok(QueueJobResponse {
            job_id,
            file_id: request.file_id,
            job_type: request.job_type,
            status: "queued".to_string(),
            message: "Job queued successfully for processing".to_string(),
        })
    }

    pub async fn queue_file_processing(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<QueueJobResponse, QueueJobError> {
        let request = QueueJobRequest {
            file_id,
            job_type: JobType::FileProcessing,
        };
        self.execute(tenant_id, request).await
    }

    pub async fn queue_url_extraction(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        url: String,
    ) -> Result<QueueJobResponse, QueueJobError> {
        // Validate URL
        if url.trim().is_empty() {
            return Err(QueueJobError::ValidationError(
                "URL cannot be empty".to_string(),
            ));
        }

        if url::Url::parse(&url).is_err() {
            return Err(QueueJobError::ValidationError(
                "Invalid URL format".to_string(),
            ));
        }

        let request = QueueJobRequest {
            file_id,
            job_type: JobType::UrlExtraction { url },
        };
        self.execute(tenant_id, request).await
    }

    pub async fn queue_youtube_extraction(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
        url: String,
    ) -> Result<QueueJobResponse, QueueJobError> {
        // Validate YouTube URL
        if url.trim().is_empty() {
            return Err(QueueJobError::ValidationError(
                "YouTube URL cannot be empty".to_string(),
            ));
        }

        let parsed_url = url::Url::parse(&url)
            .map_err(|_| QueueJobError::ValidationError("Invalid URL format".to_string()))?;

        // Check if it's a valid YouTube URL
        let is_youtube = match parsed_url.host_str() {
            Some("www.youtube.com") | Some("youtube.com") | Some("youtu.be") => true,
            _ => false,
        };

        if !is_youtube {
            return Err(QueueJobError::ValidationError(
                "URL must be a valid YouTube URL".to_string(),
            ));
        }

        let request = QueueJobRequest {
            file_id,
            job_type: JobType::YoutubeExtraction { url },
        };
        self.execute(tenant_id, request).await
    }
}
