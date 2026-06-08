use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::use_cases::search_content::SearchContentResponse;
use crate::presentation::http::dto::document_dto::DocumentWithChunksDto;

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchRequestDto {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: Option<i32>,
    pub similarity_threshold: Option<f32>,
    pub file_id: Option<Uuid>,
}

fn default_limit() -> Option<i32> {
    Some(10)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponseDto {
    pub query: String,
    pub documents: Vec<DocumentWithChunksDto>,
    pub total_documents: usize,
    pub total_chunk_matches: i32,
    pub search_time_ms: u64,
}

impl From<SearchContentResponse> for SearchResponseDto {
    fn from(response: SearchContentResponse) -> Self {
        let documents: Vec<DocumentWithChunksDto> = response
            .documents
            .into_iter()
            .map(DocumentWithChunksDto::from)
            .collect();
        Self {
            query: response.query,
            total_documents: documents.len(),
            total_chunk_matches: response.total_chunk_matches,
            documents,
            search_time_ms: response.search_time_ms,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchQueryDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub query_text: String,
    pub results_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub search_parameters: Option<serde_json::Value>,
}

impl From<crate::domain::entities::SearchQuery> for SearchQueryDto {
    fn from(q: crate::domain::entities::SearchQuery) -> Self {
        Self {
            id: q.id(),
            tenant_id: q.tenant_id(),
            query_text: q.query_text().to_string(),
            results_count: q.results_count(),
            created_at: q.created_at(),
            user_id: q.user_id().map(|s| s.to_string()),
            search_parameters: q.search_parameters().cloned(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchQueryListResponseDto {
    pub queries: Vec<SearchQueryDto>,
}
