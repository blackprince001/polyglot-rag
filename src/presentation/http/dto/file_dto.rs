use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct FileResponseDto {
    pub id: Uuid,
    pub file_name: String,
    pub file_type: Option<String>,
    pub file_size: Option<i64>,
    pub file_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub processing_status: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct PaginationDto {
    #[serde(default = "default_skip")]
    pub skip: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_skip() -> i64 {
    0
}

fn default_limit() -> i64 {
    20
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FileListResponseDto {
    pub files: Vec<FileResponseDto>,
    pub meta: PaginationMetaDto,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginationMetaDto {
    pub offset: i64,
    pub limit: i64,
    pub total: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponseDto {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub content_type: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProcessFileResponseDto {
    pub file_id: Uuid,
    pub chunks_created: i32,
    pub embeddings_created: i32,
    pub processing_time_ms: u64,
    pub message: String,
}

impl From<crate::domain::entities::File> for FileResponseDto {
    fn from(file: crate::domain::entities::File) -> Self {
        Self {
            id: file.id(),
            file_name: file.file_name().to_string(),
            file_type: file.file_type().map(|s| s.to_string()),
            file_size: file.file_size(),
            file_hash: file.file_hash().map(|h| h.as_str().to_string()),
            created_at: file.created_at().to_rfc3339(),
            updated_at: file.updated_at().to_rfc3339(),
            processing_status: file.processing_status().to_string(),
        }
    }
}

impl From<crate::application::use_cases::upload_file::UploadFileResponse> for UploadResponseDto {
    fn from(response: crate::application::use_cases::upload_file::UploadFileResponse) -> Self {
        Self {
            file_id: response.file_id,
            file_name: response.file_name,
            file_size: response.file_size,
            file_hash: response.file_hash,
            content_type: response.content_type,
            message: "File uploaded successfully".to_string(),
        }
    }
}

impl From<crate::application::use_cases::process_document::ProcessDocumentResponse>
    for ProcessFileResponseDto
{
    fn from(
        response: crate::application::use_cases::process_document::ProcessDocumentResponse,
    ) -> Self {
        Self {
            file_id: response.file_id,
            chunks_created: response.chunks_created,
            embeddings_created: response.embeddings_created,
            processing_time_ms: response.processing_time_ms,
            message: "File processed successfully".to_string(),
        }
    }
}

// DTOs for single file operations
#[derive(Debug, Serialize, ToSchema)]
pub struct FileDetailResponseDto {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub file_path: String,
    pub content_type: Option<String>,
    pub processing_status: String,
    pub created_at: String,
    pub updated_at: String,
    pub metadata: Option<serde_json::Value>,
}

impl From<crate::application::use_cases::get_file::GetFileResponse> for FileDetailResponseDto {
    fn from(response: crate::application::use_cases::get_file::GetFileResponse) -> Self {
        Self {
            file_id: response.file.id(),
            file_name: response.file.file_name().to_string(),
            file_size: response.file.file_size().unwrap_or(0),
            file_path: response.file.file_path().to_string(),
            file_hash: response
                .file
                .file_hash()
                .map(|h| h.as_str().to_string())
                .unwrap_or_default(),
            content_type: response.file.file_type().map(|ct| ct.to_string()),
            processing_status: response.file.processing_status().to_string(),
            created_at: response.file.created_at().to_rfc3339(),
            updated_at: response.file.updated_at().to_rfc3339(),
            metadata: response
                .file
                .metadata()
                .map(|m| serde_json::to_value(m).unwrap_or_default()),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RequestUploadUrlRequestDto {
    pub file_name: String,
    pub content_type: Option<String>,
    pub expiry_secs: Option<u64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RequestUploadUrlResponseDto {
    pub file_id: Uuid,
    pub file_name: String,
    pub method: String,
    pub url: Option<String>,
    pub headers: Vec<HeaderPairDto>,
    pub form_fields: Vec<HeaderPairDto>,
    pub expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HeaderPairDto {
    pub name: String,
    pub value: String,
}

impl From<(String, String)> for HeaderPairDto {
    fn from((name, value): (String, String)) -> Self {
        Self { name, value }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompleteUploadResponseDto {
    pub file_id: Uuid,
    pub job_id: Uuid,
    pub status: String,
}
