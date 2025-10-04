use async_trait::async_trait;
use pgvector::Vector;
use uuid::Uuid;

use crate::domain::entities::Embedding;

#[derive(Debug)]
pub enum EmbeddingRepositoryError {
    NotFound(Uuid),
    DatabaseError(String),
    ValidationError(String),
    // VectorError(String),
}

impl std::fmt::Display for EmbeddingRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingRepositoryError::NotFound(id) => write!(f, "Embedding not found: {}", id),
            EmbeddingRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            EmbeddingRepositoryError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            // EmbeddingRepositoryError::VectorError(msg) => write!(f, "Vector error: {}", msg),
        }
    }
}

impl std::error::Error for EmbeddingRepositoryError {}

#[derive(Debug, Clone)]
pub struct SimilaritySearchResult {
    pub embedding: Embedding,
    pub similarity_score: f32,
    pub chunk_id: Uuid,
}

#[async_trait]
pub trait EmbeddingRepository: Send + Sync {
    async fn save(&self, embedding: &Embedding) -> Result<Uuid, EmbeddingRepositoryError>;
    async fn save_batch(&self, embeddings: &[Embedding]) -> Result<Vec<Uuid>, EmbeddingRepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Embedding>, EmbeddingRepositoryError>;
    async fn find_by_chunk_id(&self, chunk_id: Uuid) -> Result<Option<Embedding>, EmbeddingRepositoryError>;
    async fn find_by_file_id(&self, file_id: Uuid) -> Result<Vec<Embedding>, EmbeddingRepositoryError>;
    async fn similarity_search(
        &self,
        query_vector: &Vector,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError>;
    async fn similarity_search_by_file(
        &self,
        query_vector: &Vector,
        file_id: Uuid,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError>;
    // async fn update(&self, embedding: &Embedding) -> Result<(), EmbeddingRepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<bool, EmbeddingRepositoryError>;
    async fn delete_by_chunk_id(&self, chunk_id: Uuid) -> Result<bool, EmbeddingRepositoryError>;
    async fn delete_by_file_id(&self, file_id: Uuid) -> Result<i64, EmbeddingRepositoryError>;
    async fn count(&self) -> Result<i64, EmbeddingRepositoryError>;
    // async fn count_by_model(&self, model_name: &str) -> Result<i64, EmbeddingRepositoryError>;
}
