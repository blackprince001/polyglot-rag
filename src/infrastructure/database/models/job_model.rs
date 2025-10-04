use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::domain::entities::{
    ProcessingJob,
    processing_job::{JobResult, JobType},
};
use crate::domain::value_objects::ProcessingStatus;
use crate::infrastructure::database::schema::processing_jobs;

#[derive(Debug, Queryable, Identifiable, Selectable)]
#[diesel(table_name = processing_jobs)]
#[diesel(primary_key(id))]
pub struct JobModel {
    pub id: Uuid,
    pub file_id: Uuid,
    pub job_type: String,
    pub job_data: Option<serde_json::Value>, // For storing URL or other job-specific data
    pub status: String,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub result_summary: Option<serde_json::Value>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = processing_jobs)]
pub struct NewJobModel {
    pub id: Option<Uuid>,
    pub file_id: Uuid,
    pub job_type: String,
    pub job_data: Option<serde_json::Value>,
    pub status: String,
    pub progress: f32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub result_summary: Option<serde_json::Value>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = processing_jobs)]
pub struct UpdateJobModel {
    pub status: Option<String>,
    pub progress: Option<f32>,
    pub started_at: Option<Option<DateTime<Utc>>>,
    pub completed_at: Option<Option<DateTime<Utc>>>,
    pub error_message: Option<Option<String>>,
    pub result_summary: Option<Option<serde_json::Value>>,
}

impl From<ProcessingJob> for NewJobModel {
    fn from(job: ProcessingJob) -> Self {
        let (job_type_str, job_data) = match job.job_type() {
            JobType::FileProcessing => ("file_processing".to_string(), None),
            JobType::UrlExtraction { url } => (
                "url_extraction".to_string(),
                Some(serde_json::json!({"url": url})),
            ),
            JobType::YoutubeExtraction { url } => (
                "youtube_extraction".to_string(),
                Some(serde_json::json!({"url": url})),
            ),
        };

        // For failed status, store error details in error_message field
        let error_message = match job.status() {
            ProcessingStatus::Failed(error) => Some(error.clone()),
            _ => job.error_message().map(|s| s.to_string()),
        };

        Self {
            id: None, // Let database generate the ID
            file_id: job.file_id(),
            job_type: job_type_str,
            job_data,
            status: job.status().to_string(),
            progress: job.progress(),
            created_at: job.created_at(),
            started_at: job.started_at(),
            completed_at: job.completed_at(),
            error_message,
            result_summary: job
                .result_summary()
                .map(|r| serde_json::to_value(r).unwrap_or_default()),
        }
    }
}

impl From<ProcessingJob> for UpdateJobModel {
    fn from(job: ProcessingJob) -> Self {
        // For failed status, store error details in error_message field
        let error_message = match job.status() {
            ProcessingStatus::Failed(error) => Some(Some(error.clone())),
            _ => job.error_message().map(|s| s.to_string()).map(Some),
        };

        Self {
            status: Some(job.status().to_string()),
            progress: Some(job.progress()),
            started_at: Some(job.started_at()),
            completed_at: Some(job.completed_at()),
            error_message,
            result_summary: Some(
                job.result_summary()
                    .map(|r| serde_json::to_value(r).unwrap_or_default()),
            ),
        }
    }
}

impl TryFrom<JobModel> for ProcessingJob {
    type Error = String;

    fn try_from(model: JobModel) -> Result<Self, Self::Error> {
        let job_type = match model.job_type.as_str() {
            "file_processing" => JobType::FileProcessing,
            "url_extraction" => {
                let url = model
                    .job_data
                    .as_ref()
                    .and_then(|data| data.get("url"))
                    .and_then(|url| url.as_str())
                    .ok_or("Missing URL in job data")?
                    .to_string();
                JobType::UrlExtraction { url }
            }
            "youtube_extraction" => {
                let url = model
                    .job_data
                    .as_ref()
                    .and_then(|data| data.get("url"))
                    .and_then(|url| url.as_str())
                    .ok_or("Missing URL in job data")?
                    .to_string();
                JobType::YoutubeExtraction { url }
            }
            _ => return Err(format!("Unknown job type: {}", model.job_type)),
        };

        let status = match model.status.as_str() {
            "pending" => ProcessingStatus::Pending,
            "processing" => ProcessingStatus::Processing,
            "completed" => ProcessingStatus::Completed,
            "failed" => {
                // Error details are stored in error_message field
                let error = model
                    .error_message
                    .as_deref()
                    .unwrap_or("Unknown error")
                    .to_string();
                ProcessingStatus::Failed(error)
            }
            s if s.starts_with("failed:") => {
                // Handle legacy format for backward compatibility
                let error = s.strip_prefix("failed:").unwrap_or(s).to_string();
                ProcessingStatus::Failed(error)
            }
            _ => return Err(format!("Unknown status: {}", model.status)),
        };

        let result_summary = if let Some(result_json) = model.result_summary {
            Some(
                serde_json::from_value::<JobResult>(result_json)
                    .map_err(|e| format!("Failed to parse result summary: {}", e))?,
            )
        } else {
            None
        };

        // Create the job using the from_database constructor with all actual database values
        let job = ProcessingJob::from_database(
            model.id,
            model.file_id,
            job_type,
            status,
            model.progress,
            model.created_at,
            model.started_at,
            model.completed_at,
            model.error_message,
            result_summary,
        );

        Ok(job)
    }
}

impl JobModel {
    pub fn is_active(&self) -> bool {
        matches!(self.status.as_str(), "pending" | "processing")
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status.as_str(), "completed") || self.status.starts_with("failed:")
    }
}
