use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::value_objects::ProcessingStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingJob {
    id: Uuid,
    file_id: Uuid,
    job_type: JobType,
    status: ProcessingStatus,
    progress: f32, // 0.0 to 1.0
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    error_message: Option<String>,
    result_summary: Option<JobResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JobType {
    FileProcessing,
    UrlExtraction { url: String },
    YoutubeExtraction { url: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobResult {
    pub chunks_created: i32,
    pub embeddings_created: i32,
    pub processing_time_ms: u64,
    pub extracted_text_length: usize,
}

impl ProcessingJob {
    pub fn new_file_processing(file_id: Uuid) -> Self {
        Self {
            id: Uuid::nil(),
            file_id,
            job_type: JobType::FileProcessing,
            status: ProcessingStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            result_summary: None,
        }
    }

    pub fn new_url_extraction(file_id: Uuid, url: String) -> Self {
        Self {
            id: Uuid::nil(),
            file_id,
            job_type: JobType::UrlExtraction { url },
            status: ProcessingStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            result_summary: None,
        }
    }

    pub fn new_youtube_extraction(file_id: Uuid, url: String) -> Self {
        Self {
            id: Uuid::nil(),
            file_id,
            job_type: JobType::YoutubeExtraction { url },
            status: ProcessingStatus::Pending,
            progress: 0.0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            result_summary: None,
        }
    }

    /// Create a ProcessingJob from database values (for repository reconstruction)
    pub fn from_database(
        id: Uuid,
        file_id: Uuid,
        job_type: JobType,
        status: ProcessingStatus,
        progress: f32,
        created_at: DateTime<Utc>,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
        error_message: Option<String>,
        result_summary: Option<JobResult>,
    ) -> Self {
        Self {
            id,
            file_id,
            job_type,
            status,
            progress,
            created_at,
            started_at,
            completed_at,
            error_message,
            result_summary,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn file_id(&self) -> Uuid {
        self.file_id
    }

    pub fn job_type(&self) -> &JobType {
        &self.job_type
    }

    pub fn status(&self) -> &ProcessingStatus {
        &self.status
    }

    pub fn progress(&self) -> f32 {
        self.progress
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn result_summary(&self) -> Option<&JobResult> {
        self.result_summary.as_ref()
    }

    // Business logic methods
    pub fn start_processing(&mut self) -> Result<(), String> {
        if !self.status.is_pending() {
            return Err(format!("Job is not in pending state: {:?}", self.status));
        }

        self.status = ProcessingStatus::Processing;
        self.started_at = Some(Utc::now());
        self.progress = 0.1;
        Ok(())
    }

    pub fn update_progress(
        &mut self,
        progress: f32,
        message: Option<String>,
    ) -> Result<(), String> {
        if !self.status.is_processing() {
            return Err("Job is not in processing state".to_string());
        }

        if progress < 0.0 || progress > 1.0 {
            return Err("Progress must be between 0.0 and 1.0".to_string());
        }

        self.progress = progress;
        if let Some(msg) = message {
            self.error_message = Some(msg); // Reusing error_message field for progress messages
        }
        Ok(())
    }

    pub fn complete_processing(&mut self, result: JobResult) -> Result<(), String> {
        if !self.status.is_processing() {
            return Err("Job is not in processing state".to_string());
        }

        self.status = ProcessingStatus::Completed;
        self.progress = 1.0;
        self.completed_at = Some(Utc::now());
        self.result_summary = Some(result);
        self.error_message = None; // Clear any progress messages
        Ok(())
    }

    pub fn fail_processing(&mut self, error: String) -> Result<(), String> {
        if !self.status.is_processing() {
            return Err("Job is not in processing state".to_string());
        }

        self.status = ProcessingStatus::Failed(error.clone());
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error);
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), String> {
        if self.status.is_terminal() {
            return Err("Cannot cancel completed or failed job".to_string());
        }

        self.status = ProcessingStatus::Failed("Cancelled by user".to_string());
        self.completed_at = Some(Utc::now());
        self.error_message = Some("Job was cancelled".to_string());
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            ProcessingStatus::Pending | ProcessingStatus::Processing
        )
    }

    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) if self.status.is_processing() => Some(Utc::now() - start),
            _ => None,
        }
    }

    pub fn estimated_completion(&self) -> Option<DateTime<Utc>> {
        if let Some(start) = self.started_at {
            if self.progress > 0.1 && self.status.is_processing() {
                let elapsed = Utc::now() - start;
                let estimated_total = elapsed.num_milliseconds() as f64 / self.progress as f64;
                let remaining = estimated_total - elapsed.num_milliseconds() as f64;
                return Some(Utc::now() + chrono::Duration::milliseconds(remaining as i64));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let file_id = Uuid::new_v4();
        let job = ProcessingJob::new_file_processing(file_id);

        assert_eq!(job.file_id(), file_id);
        assert_eq!(job.status(), &ProcessingStatus::Pending);
        assert_eq!(job.progress(), 0.0);
        assert!(job.is_active());
    }

    #[test]
    fn test_job_workflow() {
        let file_id = Uuid::new_v4();
        let mut job = ProcessingJob::new_file_processing(file_id);

        // Start processing
        assert!(job.start_processing().is_ok());
        assert_eq!(job.status(), &ProcessingStatus::Processing);
        assert!(job.started_at().is_some());

        // Update progress
        assert!(
            job.update_progress(0.5, Some("Halfway done".to_string()))
                .is_ok()
        );
        assert_eq!(job.progress(), 0.5);

        // Complete processing
        let result = JobResult {
            chunks_created: 10,
            embeddings_created: 10,
            processing_time_ms: 5000,
            extracted_text_length: 1000,
        };
        assert!(job.complete_processing(result).is_ok());
        assert_eq!(job.status(), &ProcessingStatus::Completed);
        assert!(job.completed_at().is_some());
        assert!(!job.is_active());
    }

    #[test]
    fn test_job_failure() {
        let file_id = Uuid::new_v4();
        let mut job = ProcessingJob::new_file_processing(file_id);

        job.start_processing().unwrap();
        assert!(
            job.fail_processing("Something went wrong".to_string())
                .is_ok()
        );

        if let ProcessingStatus::Failed(error) = job.status() {
            assert_eq!(error, "Something went wrong");
        } else {
            panic!("Expected failed status");
        }
    }

    #[test]
    fn test_url_extraction_job() {
        let file_id = Uuid::new_v4();
        let url = "https://example.com".to_string();
        let job = ProcessingJob::new_url_extraction(file_id, url.clone());

        if let JobType::UrlExtraction { url: job_url } = job.job_type() {
            assert_eq!(job_url, &url);
        } else {
            panic!("Expected URL extraction job type");
        }
    }
}
