use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::use_cases::{
    get_job_status::GetJobStatusResponse, queue_processing_job::QueueJobResponse,
};
use crate::domain::entities::processing_job::{JobResult, JobType, ProcessingJob};

#[derive(Debug, Serialize, ToSchema)]
pub struct JobStatusDto {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub job_type: JobTypeDto,
    pub status: String,
    pub progress: f32,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub result_summary: Option<JobResultDto>,
    pub estimated_completion: Option<String>,
    pub duration_ms: Option<i64>,
    pub is_terminal: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobTypeDto {
    pub type_name: String,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JobResultDto {
    pub chunks_created: i32,
    pub embeddings_created: i32,
    pub assets_created: i32,
    pub processing_time_ms: u64,
    pub extracted_text_length: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct QueueJobResponseDto {
    pub job_id: Uuid,
    pub file_id: Uuid,
    pub job_type: JobTypeDto,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CancelJobResponseDto {
    pub job_id: Uuid,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProcessUrlRequestDto {
    pub url: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ProcessYoutubeRequestDto {
    pub url: String,
}

impl From<GetJobStatusResponse> for JobStatusDto {
    fn from(response: GetJobStatusResponse) -> Self {
        Self::from_job_with_extras(
            response.job,
            response.estimated_completion,
            response.duration,
        )
    }
}

impl JobStatusDto {
    pub fn from_job(job: ProcessingJob) -> Self {
        Self::from_job_with_extras(job, None, None)
    }

    pub fn from_job_with_extras(
        job: ProcessingJob,
        estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
        duration: Option<chrono::Duration>,
    ) -> Self {
        let job_type = match job.job_type() {
            JobType::FileProcessing => JobTypeDto {
                type_name: "file_processing".to_string(),
                url: None,
            },
            JobType::UrlExtraction { url } => JobTypeDto {
                type_name: "url_extraction".to_string(),
                url: Some(url.clone()),
            },
            JobType::YoutubeExtraction { url } => JobTypeDto {
                type_name: "youtube_extraction".to_string(),
                url: Some(url.clone()),
            },
        };

        Self {
            job_id: job.id(),
            file_id: job.file_id(),
            job_type,
            status: job.status().to_string(),
            progress: job.progress(),
            created_at: job.created_at().to_rfc3339(),
            started_at: job.started_at().map(|dt| dt.to_rfc3339()),
            completed_at: job.completed_at().map(|dt| dt.to_rfc3339()),
            error_message: job.error_message().map(|s| s.to_string()),
            result_summary: job.result_summary().map(JobResultDto::from),
            estimated_completion: estimated_completion.map(|dt| dt.to_rfc3339()),
            duration_ms: duration.map(|d| d.num_milliseconds()),
            is_terminal: job.status().is_terminal(),
        }
    }
}

impl From<&JobResult> for JobResultDto {
    fn from(result: &JobResult) -> Self {
        Self {
            chunks_created: result.chunks_created,
            embeddings_created: result.embeddings_created,
            assets_created: result.assets_created,
            processing_time_ms: result.processing_time_ms,
            extracted_text_length: result.extracted_text_length,
        }
    }
}

impl From<QueueJobResponse> for QueueJobResponseDto {
    fn from(response: QueueJobResponse) -> Self {
        let job_type = match response.job_type {
            JobType::FileProcessing => JobTypeDto {
                type_name: "file_processing".to_string(),
                url: None,
            },
            JobType::UrlExtraction { url } => JobTypeDto {
                type_name: "url_extraction".to_string(),
                url: Some(url),
            },
            JobType::YoutubeExtraction { url } => JobTypeDto {
                type_name: "youtube_extraction".to_string(),
                url: Some(url),
            },
        };

        Self {
            job_id: response.job_id,
            file_id: response.file_id,
            job_type,
            status: response.status,
            message: response.message,
        }
    }
}

impl From<crate::application::use_cases::cancel_job::CancelJobResponse> for CancelJobResponseDto {
    fn from(response: crate::application::use_cases::cancel_job::CancelJobResponse) -> Self {
        Self {
            job_id: response.job_id,
            status: response.status,
            message: response.message,
        }
    }
}
