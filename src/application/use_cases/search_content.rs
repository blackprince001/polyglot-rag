use std::sync::Arc;

use crate::application::services::SearchService;
use crate::domain::entities::{Asset, ContentChunk, File, SearchQuery};
use crate::domain::repositories::SearchQueryRepository;

#[derive(Debug)]
pub enum SearchContentError {
    RepositoryError(String),
    ValidationError(String),
}

impl std::fmt::Display for SearchContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchContentError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
            SearchContentError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for SearchContentError {}

#[derive(Debug, Clone)]
pub struct SearchContentRequest {
    pub query: String,
    pub limit: Option<i32>,
    pub similarity_threshold: Option<f32>,
    pub file_id_filter: Option<uuid::Uuid>,
}

#[derive(Debug, Clone)]
pub struct ScoredChunk {
    pub chunk: ContentChunk,
    pub similarity_score: f32,
}

#[derive(Debug, Clone)]
pub struct DocumentMatch {
    pub file: File,
    pub chunks: Vec<ScoredChunk>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone)]
pub struct SearchContentResponse {
    pub query: String,
    pub documents: Vec<DocumentMatch>,
    pub total_chunk_matches: i32,
    pub search_time_ms: u64,
}

pub struct SearchContentUseCase {
    search_service: Arc<SearchService>,
    search_query_repository: Arc<dyn SearchQueryRepository>,
}

impl SearchContentUseCase {
    pub fn new(
        search_service: Arc<SearchService>,
        search_query_repository: Arc<dyn SearchQueryRepository>,
    ) -> Self {
        Self {
            search_service,
            search_query_repository,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: uuid::Uuid,
        request: SearchContentRequest,
    ) -> Result<SearchContentResponse, SearchContentError> {
        let start_time = std::time::Instant::now();

        if request.query.trim().is_empty() {
            return Err(SearchContentError::ValidationError(
                "Query cannot be empty".to_string(),
            ));
        }

        let limit = request.limit.unwrap_or(10);
        if limit <= 0 || limit > 100 {
            return Err(SearchContentError::ValidationError(
                "Limit must be between 1 and 100".to_string(),
            ));
        }

        let documents = self
            .search_service
            .search_content(
                tenant_id,
                &request.query,
                limit,
                request.similarity_threshold,
                request.file_id_filter,
            )
            .await
            .map_err(|e| SearchContentError::RepositoryError(e.to_string()))?;

        let total_chunk_matches = documents.iter().map(|d| d.chunks.len() as i32).sum();
        let search_time = start_time.elapsed().as_millis() as u64;

        // Best-effort persist of the search query (fire-and-forget).
        let search_query = SearchQuery::new(
            tenant_id,
            request.query.clone(),
            total_chunk_matches,
            None,
            None,
        );
        let repo = self.search_query_repository.clone();
        tokio::spawn(async move {
            if let Err(e) = repo.save(&search_query).await {
                tracing::warn!("Failed to persist search query: {}", e);
            }
        });

        Ok(SearchContentResponse {
            query: request.query,
            documents,
            total_chunk_matches,
            search_time_ms: search_time,
        })
    }
}
