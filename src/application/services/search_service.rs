use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::embedding_provider::{EmbeddingProvider, EmbeddingRequest};
use crate::application::use_cases::search_content::SearchResult;
use crate::domain::repositories::{ChunkRepository, EmbeddingRepository};

#[derive(Debug)]
pub enum SearchServiceError {
    EmbeddingError(String),
    RepositoryError(String),
}

impl std::fmt::Display for SearchServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchServiceError::EmbeddingError(msg) => write!(f, "Embedding error: {}", msg),
            SearchServiceError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for SearchServiceError {}

pub struct SearchService {
    embedding_provider: Arc<dyn EmbeddingProvider>,
    embedding_repository: Arc<dyn EmbeddingRepository>,
    chunk_repository: Arc<dyn ChunkRepository>,
}

impl SearchService {
    pub fn new(
        embedding_provider: Arc<dyn EmbeddingProvider>,
        embedding_repository: Arc<dyn EmbeddingRepository>,
        chunk_repository: Arc<dyn ChunkRepository>,
    ) -> Self {
        Self {
            embedding_provider,
            embedding_repository,
            chunk_repository,
        }
    }

    pub async fn search_content(
        &self,
        query: &str,
        limit: i32,
        similarity_threshold: Option<f32>,
        file_id_filter: Option<Uuid>,
    ) -> Result<Vec<SearchResult>, SearchServiceError> {
        // Generate embedding for the query
        let embedding_request = EmbeddingRequest {
            text: query.to_string(),
            model_name: None, // Use default model
            model_version: None,
        };

        let embedding_response = self
            .embedding_provider
            .generate_embedding(embedding_request)
            .await
            .map_err(|e| SearchServiceError::EmbeddingError(e.to_string()))?;

        // Perform similarity search
        let similarity_results = if let Some(file_id) = file_id_filter {
            self.embedding_repository
                .similarity_search_by_file(
                    &embedding_response.embedding,
                    file_id,
                    limit,
                    similarity_threshold,
                )
                .await
        } else {
            self.embedding_repository
                .similarity_search(&embedding_response.embedding, limit, similarity_threshold)
                .await
        }
        .map_err(|e| SearchServiceError::RepositoryError(e.to_string()))?;

        // Get the corresponding chunks
        let mut results = Vec::new();
        for similarity_result in similarity_results {
            if let Ok(Some(chunk)) = self
                .chunk_repository
                .find_by_id(similarity_result.chunk_id)
                .await
            {
                results.push(SearchResult {
                    chunk: chunk.clone(),
                    similarity_score: similarity_result.similarity_score,
                    file_id: chunk.file_id(), // CORRECT: Use the file_id from the chunk
                });
            }
        }

        Ok(results)
    }
}
