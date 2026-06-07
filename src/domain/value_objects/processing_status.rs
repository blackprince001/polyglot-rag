use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProcessingStatus {
    Pending,
    Processing,
    Completed,
    Failed(String),
}

impl ProcessingStatus {
    pub fn is_pending(&self) -> bool {
        matches!(self, ProcessingStatus::Pending)
    }

    pub fn is_processing(&self) -> bool {
        matches!(self, ProcessingStatus::Processing)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, ProcessingStatus::Completed)
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, ProcessingStatus::Failed(_))
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ProcessingStatus::Completed | ProcessingStatus::Failed(_)
        )
    }

    pub fn can_transition_to(&self, new_status: &ProcessingStatus) -> bool {
        match (self, new_status) {
            (ProcessingStatus::Pending, ProcessingStatus::Processing) => true,
            (ProcessingStatus::Processing, ProcessingStatus::Completed) => true,
            (ProcessingStatus::Processing, ProcessingStatus::Failed(_)) => true,
            (ProcessingStatus::Failed(_), ProcessingStatus::Pending) => true, // Allow retry
            _ => false,
        }
    }

    pub fn error_message(&self) -> Option<&str> {
        match self {
            ProcessingStatus::Failed(error) => Some(error),
            _ => None,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            ProcessingStatus::Pending => "pending".to_string(),
            ProcessingStatus::Processing => "processing".to_string(),
            ProcessingStatus::Completed => "completed".to_string(),
            ProcessingStatus::Failed(_) => "failed".to_string(), // Keep status short, store error in error_message field
        }
    }

    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(ProcessingStatus::Pending),
            "processing" => Ok(ProcessingStatus::Processing),
            "completed" => Ok(ProcessingStatus::Completed),
            "failed" => Ok(ProcessingStatus::Failed("Processing failed".to_string())), // Default error message
            s if s.starts_with("failed:") => {
                // Handle legacy format for backward compatibility
                let error = s.strip_prefix("failed:").unwrap_or("").trim();
                Ok(ProcessingStatus::Failed(error.to_string()))
            }
            _ => Err(format!("Invalid processing status: {}", s)),
        }
    }

    pub fn progress_percentage(&self) -> f32 {
        match self {
            ProcessingStatus::Pending => 0.0,
            ProcessingStatus::Processing => 50.0, // Intermediate progress
            ProcessingStatus::Completed => 100.0,
            ProcessingStatus::Failed(_) => 0.0,
        }
    }
}

impl Default for ProcessingStatus {
    fn default() -> Self {
        ProcessingStatus::Pending
    }
}

impl std::fmt::Display for ProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_checks() {
        let pending = ProcessingStatus::Pending;
        let processing = ProcessingStatus::Processing;
        let completed = ProcessingStatus::Completed;
        let failed = ProcessingStatus::Failed("error".to_string());

        assert!(pending.is_pending());
        assert!(processing.is_processing());
        assert!(completed.is_completed());
        assert!(failed.is_failed());

        assert!(!pending.is_terminal());
        assert!(!processing.is_terminal());
        assert!(completed.is_terminal());
        assert!(failed.is_terminal());
    }

    #[test]
    fn test_transitions() {
        let pending = ProcessingStatus::Pending;
        let processing = ProcessingStatus::Processing;
        let completed = ProcessingStatus::Completed;
        let failed = ProcessingStatus::Failed("error".to_string());

        // Valid transitions
        assert!(pending.can_transition_to(&processing));
        assert!(processing.can_transition_to(&completed));
        assert!(processing.can_transition_to(&failed));
        assert!(failed.can_transition_to(&pending)); // Retry

        // Invalid transitions
        assert!(!pending.can_transition_to(&completed));
        assert!(!completed.can_transition_to(&processing));
        assert!(!completed.can_transition_to(&pending));
    }

    #[test]
    fn test_error_message() {
        let failed = ProcessingStatus::Failed("Something went wrong".to_string());
        let pending = ProcessingStatus::Pending;

        assert_eq!(failed.error_message(), Some("Something went wrong"));
        assert_eq!(pending.error_message(), None);
    }

    #[test]
    fn test_string_conversion() {
        // The non-failed variants round-trip exactly.
        for status in [
            ProcessingStatus::Pending,
            ProcessingStatus::Processing,
            ProcessingStatus::Completed,
        ] {
            let parsed = ProcessingStatus::from_string(&status.to_string()).unwrap();
            assert_eq!(status, parsed);
        }

        // `to_string()` intentionally collapses `Failed(_)` to the bare "failed"
        // status; the error payload is persisted separately (the `error_message`
        // column). So the round-trip preserves the variant but not the message.
        let failed = ProcessingStatus::Failed("test error".to_string());
        let parsed = ProcessingStatus::from_string(&failed.to_string()).unwrap();
        assert!(parsed.is_failed());
    }

    #[test]
    fn test_failed_round_trip_with_inline_error() {
        // The "failed: <msg>" legacy form does carry the message back.
        let parsed = ProcessingStatus::from_string("failed: boom").unwrap();
        assert_eq!(parsed, ProcessingStatus::Failed("boom".to_string()));
    }

    #[test]
    fn test_progress_percentage() {
        assert_eq!(ProcessingStatus::Pending.progress_percentage(), 0.0);
        assert_eq!(ProcessingStatus::Processing.progress_percentage(), 50.0);
        assert_eq!(ProcessingStatus::Completed.progress_percentage(), 100.0);
        assert_eq!(
            ProcessingStatus::Failed("error".to_string()).progress_percentage(),
            0.0
        );
    }

    #[test]
    fn test_invalid_string_parsing() {
        let result = ProcessingStatus::from_string("invalid_status");
        assert!(result.is_err());
    }
}
