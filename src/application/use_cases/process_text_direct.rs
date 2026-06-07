use std::sync::Arc;
use uuid::Uuid;

use super::queue_processing_job::{QueueJobRequest, QueueProcessingJobUseCase};
use crate::application::ports::file_storage::FileStorage;
use crate::domain::entities::{File, processing_job::JobType};
use crate::domain::repositories::FileRepository;
use crate::domain::value_objects::{FileHash, FileMetadata};

/// Max bytes accepted in a single text-blob request. Larger payloads should
/// use `POST /upload` (multipart) so the body goes through the global
/// 250 MB limit. 1 MiB covers notes, articles, transcripts comfortably.
const MAX_TEXT_BLOB_BYTES: usize = 1 * 1024 * 1024;

#[derive(Debug)]
pub struct ProcessTextDirectRequest {
    pub text: String,
    pub filename: Option<String>,
    pub auto_process: bool,
}

#[derive(Debug)]
pub struct ProcessTextDirectResponse {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub filename: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug)]
pub enum ProcessTextDirectError {
    RepositoryError(String),
    StorageError(String),
    QueueError(String),
    ValidationError(String),
}

impl From<super::queue_processing_job::QueueJobError> for ProcessTextDirectError {
    fn from(error: super::queue_processing_job::QueueJobError) -> Self {
        match error {
            super::queue_processing_job::QueueJobError::RepositoryError(msg) => {
                ProcessTextDirectError::RepositoryError(msg)
            }
            super::queue_processing_job::QueueJobError::ValidationError(msg) => {
                ProcessTextDirectError::ValidationError(msg)
            }
            _ => ProcessTextDirectError::QueueError(error.to_string()),
        }
    }
}

impl std::fmt::Display for ProcessTextDirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessTextDirectError::RepositoryError(msg) => {
                write!(f, "Repository error: {}", msg)
            }
            ProcessTextDirectError::StorageError(msg) => {
                write!(f, "Storage error: {}", msg)
            }
            ProcessTextDirectError::QueueError(msg) => {
                write!(f, "Queue error: {}", msg)
            }
            ProcessTextDirectError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProcessTextDirectError {}

pub struct ProcessTextDirectUseCase {
    file_repository: Arc<dyn FileRepository>,
    file_storage: Arc<dyn FileStorage>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
}

impl ProcessTextDirectUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        file_storage: Arc<dyn FileStorage>,
        queue_job_use_case: Arc<QueueProcessingJobUseCase>,
    ) -> Self {
        Self {
            file_repository,
            file_storage,
            queue_job_use_case,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: ProcessTextDirectRequest,
    ) -> Result<ProcessTextDirectResponse, ProcessTextDirectError> {
        // Validate payload.
        if request.text.is_empty() {
            return Err(ProcessTextDirectError::ValidationError(
                "text cannot be empty".to_string(),
            ));
        }
        if request.text.len() > MAX_TEXT_BLOB_BYTES {
            return Err(ProcessTextDirectError::ValidationError(format!(
                "text exceeds maximum size of {} bytes",
                MAX_TEXT_BLOB_BYTES
            )));
        }

        // Filename: prefer caller-provided, else synthesise a timestamped name.
        let filename = request
            .filename
            .unwrap_or_else(|| format!("text-{}.txt", chrono::Utc::now().timestamp_millis()));

        let bytes = request.text.into_bytes();
        let file_size = bytes.len() as i64;
        let file_id = uuid::Uuid::new_v4();
        let stored = self
            .file_storage
            .store_file(tenant_id, file_id, &bytes, &filename, Some("text/plain"))
            .await
            .map_err(|e| ProcessTextDirectError::StorageError(e.to_string()))?;

        // Build the file entity.
        let mut metadata = FileMetadata::new();
        metadata.set_property(
            "source_type".to_string(),
            serde_json::Value::String("text_blob".to_string()),
        );
        metadata.set_property(
            "byte_size".to_string(),
            serde_json::Value::Number(file_size.into()),
        );

        let file_hash = FileHash::from_bytes(&bytes);
        let file = File::new(
            stored.key,
            filename.clone(),
            Some(file_size),
            Some("text/plain".to_string()),
            Some(file_hash),
            Some(metadata),
        );

        // Persist file metadata; the storage is already done above.
        let saved_id = self
            .file_repository
            .save(tenant_id, &file)
            .await
            .map_err(|e| ProcessTextDirectError::RepositoryError(e.to_string()))?;
        let file_id = if saved_id == file_id {
            file_id
        } else {
            // Storage and DB disagree on the id — clean up the orphan blob.
            let _ = self.file_storage.delete(tenant_id, file_id).await;
            return Err(ProcessTextDirectError::RepositoryError(format!(
                "File id mismatch after save: storage={} db={}",
                file_id, saved_id
            )));
        };

        if !request.auto_process {
            return Err(ProcessTextDirectError::ValidationError(
                "auto_process must be true for /process/text".to_string(),
            ));
        }

        let queue_request = QueueJobRequest {
            file_id,
            job_type: JobType::FileProcessing,
        };
        let job_response = self
            .queue_job_use_case
            .execute(tenant_id, queue_request)
            .await?;

        Ok(ProcessTextDirectResponse {
            job_id: job_response.job_id,
            file_id,
            filename,
            status: job_response.status,
            message: "Text blob processing started successfully".to_string(),
        })
    }
}
