use std::sync::Arc;
use uuid::Uuid;

use crate::application::use_cases::{
    queue_processing_job::{QueueJobRequest, QueueProcessingJobUseCase},
    upload_file::{UploadFileRequest, UploadFileUseCase},
};
use crate::domain::entities::processing_job::JobType;
use crate::domain::repositories::FileRepository;

#[derive(Debug)]
pub enum UploadWithProcessingError {
    UploadError(String),
    QueueError(String),
    RepositoryError(String),
}

impl std::fmt::Display for UploadWithProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UploadWithProcessingError::UploadError(msg) => write!(f, "Upload error: {}", msg),
            UploadWithProcessingError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            UploadWithProcessingError::RepositoryError(msg) => {
                write!(f, "Repository error: {}", msg)
            }
        }
    }
}

impl std::error::Error for UploadWithProcessingError {}

impl From<crate::application::use_cases::upload_file::UploadFileError>
    for UploadWithProcessingError
{
    fn from(error: crate::application::use_cases::upload_file::UploadFileError) -> Self {
        UploadWithProcessingError::UploadError(error.to_string())
    }
}

impl From<crate::application::use_cases::queue_processing_job::QueueJobError>
    for UploadWithProcessingError
{
    fn from(error: crate::application::use_cases::queue_processing_job::QueueJobError) -> Self {
        UploadWithProcessingError::QueueError(error.to_string())
    }
}

#[derive(Debug)]
pub struct UploadWithProcessingRequest {
    pub file_data: Vec<u8>,
    pub file_name: String,
    pub content_type: Option<String>,
    pub auto_process: bool,
    pub metadata: Option<crate::domain::value_objects::FileMetadata>,
}

#[derive(Debug)]
pub struct UploadWithProcessingResponse {
    pub file_id: Uuid,
    pub job_id: Option<Uuid>,
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub content_type: Option<String>,
    pub status: String,
    pub message: String,
}

pub struct UploadWithProcessingUseCase {
    upload_file_use_case: Arc<UploadFileUseCase>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
    file_repository: Arc<dyn FileRepository>,
}

impl UploadWithProcessingUseCase {
    pub fn new(
        upload_file_use_case: Arc<UploadFileUseCase>,
        queue_job_use_case: Arc<QueueProcessingJobUseCase>,
        file_repository: Arc<dyn FileRepository>,
    ) -> Self {
        Self {
            upload_file_use_case,
            queue_job_use_case,
            file_repository,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: UploadWithProcessingRequest,
    ) -> Result<UploadWithProcessingResponse, UploadWithProcessingError> {
        // Upload the file
        let upload_request = UploadFileRequest {
            file_data: request.file_data,
            file_name: request.file_name.clone(),
            content_type: request.content_type.clone(),
            metadata: request.metadata,
        };

        let upload_response = self
            .upload_file_use_case
            .execute(tenant_id, upload_request)
            .await?;

        // Verify file exists in database before queuing job (prevents race condition)
        match self
            .file_repository
            .find_by_id(tenant_id, upload_response.file_id)
            .await
        {
            Ok(Some(_file)) => {
                // File exists, continue
            }
            Ok(None) => {
                return Err(UploadWithProcessingError::RepositoryError(format!(
                    "File {} not found in database after upload",
                    upload_response.file_id
                )));
            }
            Err(e) => {
                return Err(UploadWithProcessingError::RepositoryError(format!(
                    "Failed to verify file exists after upload: {}",
                    e
                )));
            }
        }

        // Add a small delay to ensure the file save transaction is fully committed
        // and visible to other database connections
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Ensure file is fully committed to database with blocking verification
        let mut verification_attempts = 0;
        const MAX_VERIFICATION_ATTEMPTS: u32 = 10;
        const VERIFICATION_DELAY_MS: u64 = 50;

        loop {
            verification_attempts += 1;
            match self
                .file_repository
                .find_by_id(tenant_id, upload_response.file_id)
                .await
            {
                Ok(Some(_file)) => {
                    break;
                }
                Ok(None) => {
                    if verification_attempts < MAX_VERIFICATION_ATTEMPTS {
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            VERIFICATION_DELAY_MS,
                        ))
                        .await;
                    } else {
                        return Err(UploadWithProcessingError::RepositoryError(format!(
                            "File {} not committed to database after {} attempts",
                            upload_response.file_id, MAX_VERIFICATION_ATTEMPTS
                        )));
                    }
                }
                Err(e) => {
                    return Err(UploadWithProcessingError::RepositoryError(format!(
                        "Failed to verify file is committed: {}",
                        e
                    )));
                }
            }
        }

        // Queue processing job if auto_process is true
        let job_id = if request.auto_process {
            let queue_request = QueueJobRequest {
                file_id: upload_response.file_id,
                job_type: JobType::FileProcessing,
            };

            match self
                .queue_job_use_case
                .execute(tenant_id, queue_request)
                .await
            {
                Ok(queue_response) => Some(queue_response.job_id),
                Err(e) => {
                    // Log the error but don't fail the upload
                    eprintln!("Failed to queue processing job: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(UploadWithProcessingResponse {
            file_id: upload_response.file_id,
            job_id,
            file_name: upload_response.file_name,
            file_size: upload_response.file_size,
            file_hash: upload_response.file_hash,
            content_type: upload_response.content_type,
            status: if job_id.is_some() {
                "processing"
            } else {
                "uploaded"
            }
            .to_string(),
            message: if job_id.is_some() {
                "File uploaded and processing started successfully"
            } else {
                "File uploaded successfully"
            }
            .to_string(),
        })
    }
}
