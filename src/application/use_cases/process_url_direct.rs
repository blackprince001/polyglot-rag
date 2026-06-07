use std::sync::Arc;
use url::Url;
use uuid::Uuid;

use super::queue_processing_job::{QueueJobError, QueueJobRequest, QueueProcessingJobUseCase};
use crate::domain::entities::{File, processing_job::JobType};
use crate::domain::repositories::FileRepository;
use crate::domain::value_objects::{FileHash, FileMetadata};

#[derive(Debug)]
pub struct ProcessUrlDirectRequest {
    pub url: String,
    pub filename: Option<String>,
    pub auto_process: bool,
}

#[derive(Debug)]
pub struct ProcessUrlDirectResponse {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub url: String,
    pub filename: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug)]
pub enum ProcessUrlDirectError {
    InvalidUrl(String),
    RepositoryError(String),
    QueueError(String),
    ValidationError(String),
}

impl From<QueueJobError> for ProcessUrlDirectError {
    fn from(error: QueueJobError) -> Self {
        match error {
            QueueJobError::RepositoryError(msg) => ProcessUrlDirectError::RepositoryError(msg),
            QueueJobError::ValidationError(msg) => ProcessUrlDirectError::ValidationError(msg),
            _ => ProcessUrlDirectError::QueueError(error.to_string()),
        }
    }
}

impl std::fmt::Display for ProcessUrlDirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessUrlDirectError::InvalidUrl(msg) => write!(f, "Invalid URL: {}", msg),
            ProcessUrlDirectError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            ProcessUrlDirectError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            ProcessUrlDirectError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ProcessUrlDirectError {}

pub struct ProcessUrlDirectUseCase {
    file_repository: Arc<dyn FileRepository>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
}

impl ProcessUrlDirectUseCase {
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
        request: ProcessUrlDirectRequest,
    ) -> Result<ProcessUrlDirectResponse, ProcessUrlDirectError> {
        // Validate URL
        let parsed_url = Url::parse(&request.url)
            .map_err(|e| ProcessUrlDirectError::InvalidUrl(e.to_string()))?;

        // Generate filename if not provided
        let filename = request.filename.unwrap_or_else(|| {
            // Try to extract from URL
            parsed_url
                .path_segments()
                .and_then(|segments| segments.last())
                .filter(|name| !name.is_empty())
                .map(|name| name.to_string())
                .unwrap_or_else(|| {
                    // Use domain + current timestamp
                    let domain = parsed_url.host_str().unwrap_or("unknown");
                    format!("{}_webpage_{}", domain, chrono::Utc::now().timestamp())
                })
        });

        // Create file metadata
        let mut metadata = FileMetadata::new();
        metadata.set_property(
            "source_url".to_string(),
            serde_json::Value::String(request.url.clone()),
        );
        metadata.set_property(
            "extraction_type".to_string(),
            serde_json::Value::String("url".to_string()),
        );

        // For URLs, we don't store the URL as file content - the actual content
        // will be downloaded and stored during processing. Create a placeholder path.
        let placeholder_path = request.url.clone();
        let file_hash = FileHash::from_bytes(request.url.as_bytes());

        let file = File::new(
            placeholder_path,              // file_path
            filename.clone(),              // file_name
            None,                          // file_size (unknown until processed)
            Some("text/html".to_string()), // file_type
            Some(file_hash),               // file_hash
            Some(metadata),                // metadata
        );

        // Save file to repository and get the generated ID
        let file_id = self
            .file_repository
            .save(tenant_id, &file)
            .await
            .map_err(|e| ProcessUrlDirectError::RepositoryError(e.to_string()))?;

        // Queue processing job if auto_process is enabled
        let job_response = if request.auto_process {
            let queue_request = QueueJobRequest {
                file_id,
                job_type: JobType::UrlExtraction {
                    url: request.url.clone(),
                },
            };

            self.queue_job_use_case
                .execute(tenant_id, queue_request)
                .await?
        } else {
            return Err(ProcessUrlDirectError::ValidationError(
                "Auto-processing is required for direct URL processing".to_string(),
            ));
        };

        Ok(ProcessUrlDirectResponse {
            job_id: job_response.job_id,
            file_id,
            url: request.url,
            filename,
            status: job_response.status,
            message: "URL processing started successfully".to_string(),
        })
    }
}
