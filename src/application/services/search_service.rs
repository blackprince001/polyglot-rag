use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use pgvector::Vector;

use crate::application::ports::embedding_provider::{EmbeddingProvider, EmbeddingRequest};
use crate::application::use_cases::search_content::{DocumentMatch, ScoredChunk};
use crate::domain::repositories::{
    AssetRepository, ChunkRepository, EmbeddingRepository, FileRepository,
};

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

struct SimilarityHit {
    chunk_id: Uuid,
    similarity_score: f32,
}

pub struct SearchService {
    embedding_provider: Arc<dyn EmbeddingProvider>,
    embedding_repository: Arc<dyn EmbeddingRepository>,
    chunk_repository: Arc<dyn ChunkRepository>,
    file_repository: Arc<dyn FileRepository>,
    asset_repository: Arc<dyn AssetRepository>,
}

impl SearchService {
    pub fn new(
        embedding_provider: Arc<dyn EmbeddingProvider>,
        embedding_repository: Arc<dyn EmbeddingRepository>,
        chunk_repository: Arc<dyn ChunkRepository>,
        file_repository: Arc<dyn FileRepository>,
        asset_repository: Arc<dyn AssetRepository>,
    ) -> Self {
        Self {
            embedding_provider,
            embedding_repository,
            chunk_repository,
            file_repository,
            asset_repository,
        }
    }

    pub async fn search_content(
        &self,
        tenant_id: Uuid,
        query: &str,
        limit: i32,
        similarity_threshold: Option<f32>,
        file_id_filter: Option<Uuid>,
    ) -> Result<Vec<DocumentMatch>, SearchServiceError> {
        let embedding_request = EmbeddingRequest {
            text: query.to_string(),
        };
        let embedding_response = self
            .embedding_provider
            .generate_embedding(embedding_request)
            .await
            .map_err(|e| SearchServiceError::EmbeddingError(e.to_string()))?;

        self.search_with_vector(
            tenant_id,
            &embedding_response.embedding,
            limit,
            similarity_threshold,
            file_id_filter,
        )
        .await
    }

    pub async fn search_with_vector(
        &self,
        tenant_id: Uuid,
        query_vector: &Vector,
        limit: i32,
        similarity_threshold: Option<f32>,
        file_id_filter: Option<Uuid>,
    ) -> Result<Vec<DocumentMatch>, SearchServiceError> {
        let similarity_hits: Vec<SimilarityHit> = if let Some(file_id) = file_id_filter {
            self.embedding_repository
                .similarity_search_by_file(
                    tenant_id,
                    query_vector,
                    file_id,
                    limit,
                    similarity_threshold,
                )
                .await
        } else {
            self.embedding_repository
                .similarity_search(tenant_id, query_vector, limit, similarity_threshold)
                .await
        }
        .map_err(|e| SearchServiceError::RepositoryError(e.to_string()))?
        .into_iter()
        .map(|r| SimilarityHit {
            chunk_id: r.chunk_id,
            similarity_score: r.similarity_score,
        })
        .collect();

        self.hydrate_and_group(tenant_id, similarity_hits).await
    }

    async fn hydrate_and_group(
        &self,
        tenant_id: Uuid,
        hits: Vec<SimilarityHit>,
    ) -> Result<Vec<DocumentMatch>, SearchServiceError> {
        if hits.is_empty() {
            return Ok(Vec::new());
        }

        let chunk_ids: Vec<Uuid> = hits.iter().map(|h| h.chunk_id).collect();
        let chunks = self
            .chunk_repository
            .find_by_ids(tenant_id, &chunk_ids)
            .await
            .map_err(|e| SearchServiceError::RepositoryError(e.to_string()))?;
        let chunk_by_id: HashMap<Uuid, _> = chunks.into_iter().map(|c| (c.id(), c)).collect();

        let mut doc_order: Vec<Uuid> = Vec::new();
        let mut grouped: HashMap<Uuid, Vec<ScoredChunk>> = HashMap::new();
        for hit in &hits {
            let Some(chunk) = chunk_by_id.get(&hit.chunk_id) else {
                continue;
            };
            let file_id = chunk.file_id();
            grouped.entry(file_id).or_insert_with(|| {
                doc_order.push(file_id);
                Vec::new()
            });
            grouped.get_mut(&file_id).unwrap().push(ScoredChunk {
                chunk: chunk.clone(),
                similarity_score: hit.similarity_score,
            });
        }

        let files = self
            .file_repository
            .find_by_ids(tenant_id, &doc_order)
            .await
            .map_err(|e| SearchServiceError::RepositoryError(e.to_string()))?;
        let file_by_id: HashMap<Uuid, _> = files.into_iter().map(|f| (f.id(), f)).collect();

        let mut documents = Vec::new();
        for file_id in doc_order {
            let Some(file) = file_by_id.get(&file_id) else {
                continue;
            };
            // Result sets are small (capped at `limit` docs), so a per-file
            // asset lookup is acceptable. A failure degrades to no assets rather
            // than failing the whole search.
            let assets = self
                .asset_repository
                .find_by_file_id(tenant_id, file_id)
                .await
                .unwrap_or_default();
            documents.push(DocumentMatch {
                file: file.clone(),
                chunks: grouped.remove(&file_id).unwrap_or_default(),
                assets,
            });
        }

        Ok(documents)
    }
}
