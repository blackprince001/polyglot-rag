use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
pub struct FileListResponseDto {
    pub files: Vec<FileResponseDto>,
    pub meta: PaginationMetaDto,
}

#[derive(Debug, Serialize)]
pub struct PaginationMetaDto {
    pub offset: i64,
    pub limit: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct UploadResponseDto {
    pub file_id: Uuid,
    pub file_name: String,
    pub file_size: i64,
    pub file_hash: String,
    pub content_type: Option<String>,
    pub message: String,
}

#[derive(Debug, Serialize)]
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
#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct ContentChunkDto {
    pub chunk_id: Uuid,
    pub file_id: Uuid,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub word_count: Option<i32>,
    pub page_number: Option<i32>,
    pub section_path: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct FileChunksResponseDto {
    pub file_id: Uuid,
    pub chunks: Vec<ContentChunkDto>,
    pub meta: PaginationMetaDto,
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

impl From<&crate::domain::entities::ContentChunk> for ContentChunkDto {
    fn from(chunk: &crate::domain::entities::ContentChunk) -> Self {
        Self {
            chunk_id: chunk.id(),
            file_id: chunk.file_id(),
            chunk_text: chunk.chunk_text().to_string(),
            chunk_index: chunk.chunk_index(),
            word_count: Some(chunk.word_count() as i32),
            page_number: chunk.page_number(),
            section_path: chunk.section_path().map(|s| s.to_string()),
            created_at: chunk.created_at().to_rfc3339(),
        }
    }
}

impl From<crate::application::use_cases::get_file_chunks::GetFileChunksResponse>
    for FileChunksResponseDto
{
    fn from(
        response: crate::application::use_cases::get_file_chunks::GetFileChunksResponse,
    ) -> Self {
        Self {
            file_id: response.file_id,
            chunks: response.chunks.iter().map(ContentChunkDto::from).collect(),
            meta: PaginationMetaDto {
                offset: response.skip,
                limit: response.limit,
                total: response.total_chunks,
            },
        }
    }
}
