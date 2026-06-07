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
