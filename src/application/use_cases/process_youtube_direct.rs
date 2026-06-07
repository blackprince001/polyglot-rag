use std::sync::Arc;
use url::Url;
use uuid::Uuid;

use super::queue_processing_job::{QueueJobError, QueueJobRequest, QueueProcessingJobUseCase};
use crate::domain::entities::{File, processing_job::JobType};
use crate::domain::repositories::FileRepository;
use crate::domain::value_objects::{FileHash, FileMetadata};

#[derive(Debug)]
pub struct ProcessYoutubeDirectRequest {
    pub url: String,
    pub filename: Option<String>,
    pub extract_timestamps: bool,
    pub language_preference: Vec<String>,
    pub auto_process: bool,
}

#[derive(Debug)]
pub struct ProcessYoutubeDirectResponse {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub url: String,
    pub filename: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug)]
pub enum ProcessYoutubeDirectError {
    InvalidUrl(String),
    RepositoryError(String),
    QueueError(String),
    ValidationError(String),
}

impl From<QueueJobError> for ProcessYoutubeDirectError {
    fn from(error: QueueJobError) -> Self {
        match error {
            QueueJobError::RepositoryError(msg) => ProcessYoutubeDirectError::RepositoryError(msg),
            QueueJobError::ValidationError(msg) => ProcessYoutubeDirectError::ValidationError(msg),
            _ => ProcessYoutubeDirectError::QueueError(error.to_string()),
        }
    }
}

impl std::fmt::Display for ProcessYoutubeDirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessYoutubeDirectError::InvalidUrl(msg) => write!(f, "Invalid YouTube URL: {}", msg),
            ProcessYoutubeDirectError::RepositoryError(msg) => {
                write!(f, "Repository error: {}", msg)
            }
            ProcessYoutubeDirectError::QueueError(msg) => write!(f, "Queue error: {}", msg),
            ProcessYoutubeDirectError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ProcessYoutubeDirectError {}

pub struct ProcessYoutubeDirectUseCase {
    file_repository: Arc<dyn FileRepository>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
}

impl ProcessYoutubeDirectUseCase {
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
        request: ProcessYoutubeDirectRequest,
    ) -> Result<ProcessYoutubeDirectResponse, ProcessYoutubeDirectError> {
        // Validate YouTube URL
        let parsed_url = Url::parse(&request.url)
            .map_err(|e| ProcessYoutubeDirectError::InvalidUrl(e.to_string()))?;

        let video_id = self.extract_video_id(&parsed_url)?;

        // Generate filename if not provided
        let filename = request
            .filename
            .unwrap_or_else(|| format!("youtube_video_{}", video_id));

        // Create file metadata
        let mut metadata = FileMetadata::new();
        metadata.set_property(
            "source_url".to_string(),
            serde_json::Value::String(request.url.clone()),
        );
        metadata.set_property(
            "video_id".to_string(),
            serde_json::Value::String(video_id.clone()),
        );
        metadata.set_property(
            "extraction_type".to_string(),
            serde_json::Value::String("youtube".to_string()),
        );
        metadata.set_property(
            "extract_timestamps".to_string(),
            serde_json::Value::Bool(request.extract_timestamps),
        );
        metadata.set_property(
            "language_preference".to_string(),
            serde_json::Value::Array(
                request
                    .language_preference
                    .iter()
                    .map(|lang| serde_json::Value::String(lang.clone()))
                    .collect(),
            ),
        );

        // For YouTube URLs, we don't store the URL as file content - the actual transcript
        // will be downloaded and stored during processing. Create a placeholder path.
        let placeholder_path = request.url.clone();
        let file_hash = FileHash::from_bytes(request.url.as_bytes());

        let file = File::new(
            placeholder_path,                     // file_path
            filename.clone(),                     // file_name
            None,                                 // file_size (unknown until processed)
            Some("text/youtube-url".to_string()), // file_type (matches what extractor expects)
            Some(file_hash),                      // file_hash
            Some(metadata),                       // metadata
        );

        // Save file to repository and get the generated ID
        let file_id = self
            .file_repository
            .save(tenant_id, &file)
            .await
            .map_err(|e| ProcessYoutubeDirectError::RepositoryError(e.to_string()))?;

        // Queue processing job if auto_process is enabled
        let job_response = if request.auto_process {
            let queue_request = QueueJobRequest {
                file_id,
                job_type: JobType::YoutubeExtraction {
                    url: request.url.clone(),
                },
            };

            self.queue_job_use_case
                .execute(tenant_id, queue_request)
                .await?
        } else {
            return Err(ProcessYoutubeDirectError::ValidationError(
                "Auto-processing is required for direct YouTube processing".to_string(),
            ));
        };

        Ok(ProcessYoutubeDirectResponse {
            job_id: job_response.job_id,
            file_id,
            url: request.url,
            filename,
            status: job_response.status,
            message: "YouTube transcript extraction started successfully".to_string(),
        })
    }

    fn extract_video_id(&self, url: &Url) -> Result<String, ProcessYoutubeDirectError> {
        // Handle different YouTube URL formats
        match url.host_str() {
            Some("www.youtube.com") | Some("youtube.com") => {
                // Standard format: https://www.youtube.com/watch?v=VIDEO_ID
                if let Some(_) = url.query() {
                    for (key, value) in url.query_pairs() {
                        if key == "v" {
                            return Ok(value.to_string());
                        }
                    }
                }
                Err(ProcessYoutubeDirectError::InvalidUrl(
                    "Could not extract video ID from YouTube URL".to_string(),
                ))
            }
            Some("youtu.be") => {
                // Short format: https://youtu.be/VIDEO_ID
                if let Some(path) = url.path_segments() {
                    if let Some(video_id) = path.last() {
                        return Ok(video_id.to_string());
                    }
                }
                Err(ProcessYoutubeDirectError::InvalidUrl(
                    "Could not extract video ID from short YouTube URL".to_string(),
                ))
            }
            _ => Err(ProcessYoutubeDirectError::InvalidUrl(
                "Not a valid YouTube URL".to_string(),
            )),
        }
    }
}
