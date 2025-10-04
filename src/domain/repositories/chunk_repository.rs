use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::ContentChunk;

#[derive(Debug)]
pub enum ChunkRepositoryError {
    // NotFound(Uuid),
    DatabaseError(String),
    // ValidationError(String),
}

impl std::fmt::Display for ChunkRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // ChunkRepositoryError::NotFound(id) => write!(f, "Chunk not found: {}", id),
            ChunkRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            // ChunkRepositoryError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for ChunkRepositoryError {}

#[async_trait]
pub trait ChunkRepository: Send + Sync {
    async fn save(&self, chunk: &ContentChunk) -> Result<Uuid, ChunkRepositoryError>;
    async fn save_batch(&self, chunks: &[ContentChunk]) -> Result<Vec<Uuid>, ChunkRepositoryError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<ContentChunk>, ChunkRepositoryError>;
    // async fn find_by_file_id(&self, file_id: Uuid) -> Result<Vec<ContentChunk>, ChunkRepositoryError>;
    async fn find_by_file_id_paginated(
        &self,
        file_id: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<ContentChunk>, ChunkRepositoryError>;
    async fn delete(&self, id: Uuid) -> Result<bool, ChunkRepositoryError>;
    async fn delete_by_file_id(&self, file_id: Uuid) -> Result<i64, ChunkRepositoryError>;
    async fn count_by_file_id(&self, file_id: Uuid) -> Result<i64, ChunkRepositoryError>;
}
