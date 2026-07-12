use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::value_objects::{FileHash, FileMetadata, ProcessingStatus};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    id: Uuid,
    file_path: String,
    file_name: String,
    file_size: Option<i64>,
    file_type: Option<String>,
    file_hash: Option<FileHash>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    metadata: Option<FileMetadata>,
    processing_status: ProcessingStatus,
}

impl File {
    pub fn new(
        file_path: String,
        file_name: String,
        file_size: Option<i64>,
        file_type: Option<String>,
        file_hash: Option<FileHash>,
        metadata: Option<FileMetadata>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::nil(), // Will be set by database
            file_path,
            file_name,
            file_size,
            file_type,
            file_hash,
            created_at: now,
            updated_at: now,
            metadata,
            processing_status: ProcessingStatus::Pending,
        }
    }

    /// Like `new`, but carries a caller-generated id so the database row matches
    /// an id that was already used for blob storage.
    pub fn new_with_id(
        id: Uuid,
        file_path: String,
        file_name: String,
        file_size: Option<i64>,
        file_type: Option<String>,
        file_hash: Option<FileHash>,
        metadata: Option<FileMetadata>,
    ) -> Self {
        let mut file = Self::new(
            file_path, file_name, file_size, file_type, file_hash, metadata,
        );
        file.id = id;
        file
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: Uuid,
        file_path: String,
        file_name: String,
        file_size: Option<i64>,
        file_type: Option<String>,
        file_hash: Option<FileHash>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        metadata: Option<FileMetadata>,
        processing_status: ProcessingStatus,
    ) -> Self {
        Self {
            id,
            file_path,
            file_name,
            file_size,
            file_type,
            file_hash,
            created_at,
            updated_at,
            metadata,
            processing_status,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn file_size(&self) -> Option<i64> {
        self.file_size
    }

    pub fn file_type(&self) -> Option<&str> {
        self.file_type.as_deref()
    }

    pub fn file_hash(&self) -> Option<&FileHash> {
        self.file_hash.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub fn metadata(&self) -> Option<&FileMetadata> {
        self.metadata.as_ref()
    }

    pub fn processing_status(&self) -> &ProcessingStatus {
        &self.processing_status
    }

    pub fn start_processing(&mut self) -> Result<(), String> {
        match self.processing_status {
            ProcessingStatus::Pending => {
                self.processing_status = ProcessingStatus::Processing;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err("File is not in pending state".to_string()),
        }
    }

    pub fn complete_processing(&mut self) -> Result<(), String> {
        match self.processing_status {
            ProcessingStatus::Processing => {
                self.processing_status = ProcessingStatus::Completed;
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err("File is not being processed".to_string()),
        }
    }

    pub fn fail_processing(&mut self, error: String) -> Result<(), String> {
        match self.processing_status {
            ProcessingStatus::Processing => {
                self.processing_status = ProcessingStatus::Failed(error);
                self.updated_at = Utc::now();
                Ok(())
            }
            _ => Err("File is not being processed".to_string()),
        }
    }

    pub fn is_processable(&self) -> bool {
        matches!(self.processing_status, ProcessingStatus::Pending)
    }

    pub fn is_processed(&self) -> bool {
        matches!(self.processing_status, ProcessingStatus::Completed)
    }

    pub fn update_metadata(&mut self, metadata: FileMetadata) {
        self.metadata = Some(metadata);
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_creation() {
        let file = File::new(
            "/path/to/file.pdf".to_string(),
            "file.pdf".to_string(),
            Some(1024),
            Some("application/pdf".to_string()),
            None,
            None,
        );

        assert_eq!(file.file_name(), "file.pdf");
        assert_eq!(file.file_size(), Some(1024));
        assert_eq!(file.processing_status(), &ProcessingStatus::Pending);
        assert!(file.is_processable());
    }

    #[test]
    fn test_processing_workflow() {
        let mut file = File::new(
            "/path/to/file.pdf".to_string(),
            "file.pdf".to_string(),
            Some(1024),
            Some("application/pdf".to_string()),
            None,
            None,
        );

        assert!(file.start_processing().is_ok());
        assert_eq!(file.processing_status(), &ProcessingStatus::Processing);

        assert!(file.complete_processing().is_ok());
        assert_eq!(file.processing_status(), &ProcessingStatus::Completed);
        assert!(file.is_processed());
    }

    #[test]
    fn test_processing_failure() {
        let mut file = File::new(
            "/path/to/file.pdf".to_string(),
            "file.pdf".to_string(),
            Some(1024),
            Some("application/pdf".to_string()),
            None,
            None,
        );

        file.start_processing().unwrap();
        assert!(file.fail_processing("Processing error".to_string()).is_ok());

        if let ProcessingStatus::Failed(error) = file.processing_status() {
            assert_eq!(error, "Processing error");
        } else {
            panic!("Expected failed status");
        }
    }
}
