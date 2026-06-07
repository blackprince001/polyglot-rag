use async_trait::async_trait;
use pgvector::Vector;
use uuid::Uuid;

use crate::domain::entities::Embedding;

#[derive(Debug)]
pub enum EmbeddingRepositoryError {
    DatabaseError(String),
    ValidationError(String),
}

impl std::fmt::Display for EmbeddingRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            EmbeddingRepositoryError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
        }
    }
}

impl std::error::Error for EmbeddingRepositoryError {}

#[derive(Debug, Clone)]
pub struct SimilaritySearchResult {
    pub chunk_id: Uuid,
    pub similarity_score: f32,
}

#[async_trait]
pub trait EmbeddingRepository: Send + Sync {
    async fn save_batch(
        &self,
        tenant_id: Uuid,
        embeddings: &[Embedding],
    ) -> Result<Vec<Uuid>, EmbeddingRepositoryError>;
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError>;
    async fn find_by_chunk_id(
        &self,
        tenant_id: Uuid,
        chunk_id: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError>;
    async fn find_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<Embedding>, EmbeddingRepositoryError>;
    async fn similarity_search(
        &self,
        tenant_id: Uuid,
        query_vector: &Vector,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError>;
    async fn similarity_search_by_file(
        &self,
        tenant_id: Uuid,
        query_vector: &Vector,
        file_id: Uuid,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError>;
    async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, EmbeddingRepositoryError>;
    async fn delete_by_chunk_id(
        &self,
        tenant_id: Uuid,
        chunk_id: Uuid,
    ) -> Result<bool, EmbeddingRepositoryError>;
    async fn delete_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<i64, EmbeddingRepositoryError>;
    async fn count(&self, tenant_id: Uuid) -> Result<i64, EmbeddingRepositoryError>;
}
