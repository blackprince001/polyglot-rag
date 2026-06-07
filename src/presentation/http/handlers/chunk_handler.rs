use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::use_cases::GetFileChunksUseCase;
use crate::application::use_cases::get_file_chunks::{GetFileChunksError, GetFileChunksRequest};
use crate::domain::repositories::ChunkRepository;
use crate::presentation::http::dto::document_dto::{DocumentChunkDto, DocumentWithChunksDto};
use crate::presentation::http::dto::{ApiResponse, PaginationDto};
use crate::presentation::http::middleware::TenantContext;

pub struct ChunkHandler {
    chunk_repository: Arc<dyn ChunkRepository>,
    get_file_chunks_use_case: Arc<GetFileChunksUseCase>,
}

impl ChunkHandler {
    pub fn new(
        chunk_repository: Arc<dyn ChunkRepository>,
        get_file_chunks_use_case: Arc<GetFileChunksUseCase>,
    ) -> Self {
        Self {
            chunk_repository,
            get_file_chunks_use_case,
        }
    }

    pub async fn get_chunk(
        State(handler): State<Arc<ChunkHandler>>,
        tenant: TenantContext,
        Path(chunk_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .chunk_repository
            .find_by_id(tenant.tenant_id, chunk_id)
            .await
        {
            Ok(Some(chunk)) => {
                let dto = DocumentChunkDto {
                    chunk_id: chunk.id(),
                    chunk_text: chunk.chunk_text().to_string(),
                    chunk_index: chunk.chunk_index(),
                    page_number: chunk.page_number(),
                    section_path: chunk.section_path().map(|s| s.to_string()),
                    similarity_score: None,
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Ok(None) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "CHUNK_NOT_FOUND".to_string(),
                    format!("Chunk with ID {} not found", chunk_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "DATABASE_ERROR".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn get_chunks_by_file(
        State(handler): State<Arc<ChunkHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
        Query(pagination): Query<PaginationDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = GetFileChunksRequest {
            file_id,
            skip: Some(pagination.skip),
            limit: Some(pagination.limit),
        };

        match handler
            .get_file_chunks_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto =
                    DocumentWithChunksDto::from_file_and_chunks(&response.file, &response.chunks)
                        .with_assets(&response.assets);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(GetFileChunksError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "FILE_NOT_FOUND".to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "CHUNKS_NOT_FOUND".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn get_chunk_count_by_file(
        State(handler): State<Arc<ChunkHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .chunk_repository
            .count_by_file_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(count) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "file_id": file_id,
                    "chunk_count": count
                }))),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "COUNT_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn delete_chunk(
        State(handler): State<Arc<ChunkHandler>>,
        tenant: TenantContext,
        Path(chunk_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .chunk_repository
            .delete(tenant.tenant_id, chunk_id)
            .await
        {
            Ok(true) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(
                    "Chunk deleted successfully".to_string(),
                )),
            )),
            Ok(false) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "CHUNK_NOT_FOUND".to_string(),
                    format!("Chunk with ID {} not found", chunk_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "DELETE_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn delete_chunks_by_file(
        State(handler): State<Arc<ChunkHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .chunk_repository
            .delete_by_file_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(count) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "file_id": file_id,
                    "deleted_chunks": count
                }))),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "DELETE_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }
}
