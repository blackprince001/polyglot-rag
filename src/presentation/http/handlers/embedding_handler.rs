use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use pgvector::Vector;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::services::SearchService;
use crate::domain::repositories::EmbeddingRepository;
use crate::presentation::http::dto::ApiResponse;
use crate::presentation::http::dto::document_dto::DocumentWithChunksDto;
use crate::presentation::http::dto::error_code::ErrorCode;
use crate::presentation::http::middleware::TenantContext;

#[derive(serde::Deserialize, ToSchema)]
pub struct SimilaritySearchRequest {
    pub query_vector: Vec<f32>,
    pub limit: Option<i32>,
    pub similarity_threshold: Option<f32>,
    pub file_id: Option<Uuid>,
}

#[derive(serde::Serialize, ToSchema)]
pub struct SimilaritySearchResponse {
    pub documents: Vec<DocumentWithChunksDto>,
    pub total_documents: usize,
    pub total_chunk_matches: i32,
}

pub struct EmbeddingHandler {
    embedding_repository: Arc<dyn EmbeddingRepository>,
    search_service: Arc<SearchService>,
}

impl EmbeddingHandler {
    pub fn new(
        embedding_repository: Arc<dyn EmbeddingRepository>,
        search_service: Arc<SearchService>,
    ) -> Self {
        Self {
            embedding_repository,
            search_service,
        }
    }

    pub async fn get_embedding(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(embedding_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .find_by_id(tenant.tenant_id, embedding_id)
            .await
        {
            Ok(Some(embedding)) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "id": embedding.id(),
                    "chunk_id": embedding.content_chunk_id(),
                    "model_name": embedding.model_name(),
                    "model_version": embedding.model_version(),
                    "vector_dimension": embedding.embedding().as_slice().len(),
                    "created_at": embedding.generated_at().to_rfc3339()
                }))),
            )),
            Ok(None) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::EmbeddingNotFound.as_str().to_string(),
                    format!("Embedding with ID {} not found", embedding_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("database_error", e)),
            )),
        }
    }

    pub async fn get_embedding_by_chunk(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(chunk_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .find_by_chunk_id(tenant.tenant_id, chunk_id)
            .await
        {
            Ok(Some(embedding)) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "id": embedding.id(),
                    "chunk_id": embedding.content_chunk_id(),
                    "model_name": embedding.model_name(),
                    "model_version": embedding.model_version(),
                    "vector_dimension": embedding.embedding().as_slice().len(),
                    "created_at": embedding.generated_at().to_rfc3339()
                }))),
            )),
            Ok(None) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::EmbeddingNotFound.as_str().to_string(),
                    format!("No embedding found for chunk ID {}", chunk_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("database_error", e)),
            )),
        }
    }

    pub async fn get_embeddings_by_file(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .find_by_file_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(embeddings) => {
                let embeddings_dto: Vec<serde_json::Value> = embeddings
                    .into_iter()
                    .map(|e| {
                        serde_json::json!({
                            "id": e.id(),
                            "chunk_id": e.content_chunk_id(),
                            "model_name": e.model_name(),
                            "model_version": e.model_version(),
                            "vector_dimension": e.embedding().as_slice().len(),
                            "created_at": e.generated_at().to_rfc3339()
                        })
                    })
                    .collect();

                Ok((
                    StatusCode::OK,
                    Json(ApiResponse::success(serde_json::json!({
                        "file_id": file_id,
                        "embeddings": embeddings_dto,
                        "count": embeddings_dto.len()
                    }))),
                ))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("database_error", e)),
            )),
        }
    }

    pub async fn similarity_search(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Json(request): Json<SimilaritySearchRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let limit = request.limit.unwrap_or(10);
        if !(1..=100).contains(&limit) {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::InvalidLimit.as_str().to_string(),
                    "limit must be between 1 and 100".to_string(),
                    None,
                )),
            ));
        }
        let query_vector = Vector::from(request.query_vector);

        let documents = match handler
            .search_service
            .search_with_vector(
                tenant.tenant_id,
                &query_vector,
                limit,
                request.similarity_threshold,
                request.file_id,
            )
            .await
        {
            Ok(documents) => documents,
            Err(e) => {
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::internal_error("search_failed", e)),
                ));
            }
        };

        let total_chunk_matches = documents.iter().map(|d| d.chunks.len() as i32).sum();
        let documents_dto: Vec<DocumentWithChunksDto> = documents
            .into_iter()
            .map(DocumentWithChunksDto::from)
            .collect();
        let total_documents = documents_dto.len();

        let response = SimilaritySearchResponse {
            total_documents,
            total_chunk_matches,
            documents: documents_dto,
        };

        Ok((StatusCode::OK, Json(ApiResponse::success(response))))
    }

    pub async fn delete_embedding(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(embedding_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .delete(tenant.tenant_id, embedding_id)
            .await
        {
            Ok(true) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(
                    "Embedding deleted successfully".to_string(),
                )),
            )),
            Ok(false) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::EmbeddingNotFound.as_str().to_string(),
                    format!("Embedding with ID {} not found", embedding_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("delete_failed", e)),
            )),
        }
    }

    pub async fn delete_embeddings_by_chunk(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(chunk_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .delete_by_chunk_id(tenant.tenant_id, chunk_id)
            .await
        {
            Ok(true) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(
                    "Embeddings deleted successfully".to_string(),
                )),
            )),
            Ok(false) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::EmbeddingNotFound.as_str().to_string(),
                    format!("No embeddings found for chunk ID {}", chunk_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("delete_failed", e)),
            )),
        }
    }

    pub async fn delete_embeddings_by_file(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .embedding_repository
            .delete_by_file_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(count) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "file_id": file_id,
                    "deleted_embeddings": count
                }))),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("delete_failed", e)),
            )),
        }
    }

    pub async fn get_embedding_count(
        State(handler): State<Arc<EmbeddingHandler>>,
        tenant: TenantContext,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.embedding_repository.count(tenant.tenant_id).await {
            Ok(count) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "count": count
                }))),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("count_failed", e)),
            )),
        }
    }
}
