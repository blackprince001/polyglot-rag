use serde::Serialize;
use utoipa::ToSchema;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // Generic / cross-cutting
    /// An unexpected server-side error. The real cause is logged, never returned.
    Internal,
    ValidationError,
    InvalidRequest,
    Unauthorized,
    AuthError,
    RepositoryError,

    // Files & uploads
    FileNotFound,
    FileLookupFailed,
    FileNotProcessable,
    NoFileProvided,
    UploadValidationFailed,
    UploadFailed,
    UploadWithProcessingValidationFailed,
    UploadWithProcessingFailed,
    UploadUrlFailed,
    CompleteUploadFailed,
    UpdateFailed,
    DeleteFailed,
    CountFailed,
    ListValidationFailed,
    ListFailed,
    StreamFailed,
    PresignFailed,

    // Assets
    AssetNotFound,
    AssetLookupFailed,

    // Processing & jobs
    ProcessingFailed,
    JobNotFound,
    JobLookupFailed,
    JobNotCancellable,
    CancelFailed,
    QueueValidationFailed,
    QueueFailed,

    // Content processing
    EmptyUrl,
    EmptyText,
    InvalidUrl,
    InvalidYoutubeUrl,
    FetchFailed,
    QueueError,

    // Chunks & embeddings
    ChunkNotFound,
    ChunksNotFound,
    InvalidLimit,
    EmbeddingNotFound,

    // Search
    EmptyQuery,
    SearchValidationFailed,
    SearchFailed,

    // Tenants & API keys
    TenantNotFound,
    TenantLookupFailed,
    CreateTenantFailed,
    ListTenantsFailed,
    ApiKeyNotFound,
    CreateApiKeyFailed,
    ListApiKeysFailed,
    RevokeApiKeyFailed,
    ManagementDisabled,

    // Storage / database
    DatabaseError,
    StorageError,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        use ErrorCode::*;
        match self {
            Internal => "INTERNAL",
            ValidationError => "VALIDATION_ERROR",
            InvalidRequest => "INVALID_REQUEST",
            Unauthorized => "UNAUTHORIZED",
            FileNotFound => "FILE_NOT_FOUND",
            FileLookupFailed => "FILE_LOOKUP_FAILED",
            FileNotProcessable => "FILE_NOT_PROCESSABLE",
            NoFileProvided => "NO_FILE_PROVIDED",
            UploadValidationFailed => "UPLOAD_VALIDATION_FAILED",
            UploadFailed => "UPLOAD_FAILED",
            UploadWithProcessingValidationFailed => "UPLOAD_WITH_PROCESSING_VALIDATION_FAILED",
            UploadWithProcessingFailed => "UPLOAD_WITH_PROCESSING_FAILED",
            UploadUrlFailed => "UPLOAD_URL_FAILED",
            CompleteUploadFailed => "COMPLETE_UPLOAD_FAILED",
            UpdateFailed => "UPDATE_FAILED",
            DeleteFailed => "DELETE_FAILED",
            CountFailed => "COUNT_FAILED",
            ListValidationFailed => "LIST_VALIDATION_FAILED",
            ListFailed => "LIST_FAILED",
            StreamFailed => "STREAM_FAILED",
            PresignFailed => "PRESIGN_FAILED",
            AssetNotFound => "ASSET_NOT_FOUND",
            AssetLookupFailed => "ASSET_LOOKUP_FAILED",
            ProcessingFailed => "PROCESSING_FAILED",
            JobNotFound => "JOB_NOT_FOUND",
            JobLookupFailed => "JOB_LOOKUP_FAILED",
            JobNotCancellable => "JOB_NOT_CANCELLABLE",
            CancelFailed => "CANCEL_FAILED",
            QueueValidationFailed => "QUEUE_VALIDATION_FAILED",
            QueueFailed => "QUEUE_FAILED",
            EmptyUrl => "EMPTY_URL",
            EmptyText => "EMPTY_TEXT",
            InvalidYoutubeUrl => "INVALID_YOUTUBE_URL",
            FetchFailed => "FETCH_FAILED",
            ChunkNotFound => "CHUNK_NOT_FOUND",
            ChunksNotFound => "CHUNKS_NOT_FOUND",
            InvalidLimit => "INVALID_LIMIT",
            EmbeddingNotFound => "EMBEDDING_NOT_FOUND",
            EmptyQuery => "EMPTY_QUERY",
            SearchValidationFailed => "SEARCH_VALIDATION_FAILED",
            SearchFailed => "SEARCH_FAILED",
            TenantNotFound => "TENANT_NOT_FOUND",
            TenantLookupFailed => "TENANT_LOOKUP_FAILED",
            CreateTenantFailed => "CREATE_TENANT_FAILED",
            ListTenantsFailed => "LIST_TENANTS_FAILED",
            ApiKeyNotFound => "API_KEY_NOT_FOUND",
            CreateApiKeyFailed => "CREATE_API_KEY_FAILED",
            ListApiKeysFailed => "LIST_API_KEYS_FAILED",
            RevokeApiKeyFailed => "REVOKE_API_KEY_FAILED",
            ManagementDisabled => "MANAGEMENT_DISABLED",
            DatabaseError => "DATABASE_ERROR",
            StorageError => "STORAGE_ERROR",
            AuthError => "AUTH_ERROR",
            RepositoryError => "REPOSITORY_ERROR",
            InvalidUrl => "INVALID_URL",
            QueueError => "QUEUE_ERROR",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_matches_serde_representation() {
        let all = [
            ErrorCode::Internal,
            ErrorCode::ValidationError,
            ErrorCode::InvalidRequest,
            ErrorCode::Unauthorized,
            ErrorCode::FileNotFound,
            ErrorCode::FileLookupFailed,
            ErrorCode::FileNotProcessable,
            ErrorCode::NoFileProvided,
            ErrorCode::UploadValidationFailed,
            ErrorCode::UploadFailed,
            ErrorCode::UploadWithProcessingValidationFailed,
            ErrorCode::UploadWithProcessingFailed,
            ErrorCode::UploadUrlFailed,
            ErrorCode::CompleteUploadFailed,
            ErrorCode::UpdateFailed,
            ErrorCode::DeleteFailed,
            ErrorCode::CountFailed,
            ErrorCode::ListValidationFailed,
            ErrorCode::ListFailed,
            ErrorCode::StreamFailed,
            ErrorCode::PresignFailed,
            ErrorCode::AssetNotFound,
            ErrorCode::AssetLookupFailed,
            ErrorCode::ProcessingFailed,
            ErrorCode::JobNotFound,
            ErrorCode::JobLookupFailed,
            ErrorCode::JobNotCancellable,
            ErrorCode::CancelFailed,
            ErrorCode::QueueValidationFailed,
            ErrorCode::QueueFailed,
            ErrorCode::EmptyUrl,
            ErrorCode::EmptyText,
            ErrorCode::InvalidYoutubeUrl,
            ErrorCode::FetchFailed,
            ErrorCode::ChunkNotFound,
            ErrorCode::ChunksNotFound,
            ErrorCode::InvalidLimit,
            ErrorCode::EmbeddingNotFound,
            ErrorCode::EmptyQuery,
            ErrorCode::SearchValidationFailed,
            ErrorCode::SearchFailed,
            ErrorCode::TenantNotFound,
            ErrorCode::TenantLookupFailed,
            ErrorCode::CreateTenantFailed,
            ErrorCode::ListTenantsFailed,
            ErrorCode::ApiKeyNotFound,
            ErrorCode::CreateApiKeyFailed,
            ErrorCode::ListApiKeysFailed,
            ErrorCode::RevokeApiKeyFailed,
            ErrorCode::ManagementDisabled,
            ErrorCode::DatabaseError,
            ErrorCode::StorageError,
            ErrorCode::AuthError,
            ErrorCode::RepositoryError,
            ErrorCode::InvalidUrl,
            ErrorCode::QueueError,
        ];
        for code in all {
            let serde_form = serde_json::to_string(&code).unwrap();
            let serde_form = serde_form.trim_matches('"');
            assert_eq!(serde_form, code.as_str(), "mismatch for {:?}", code);
        }
    }
}
