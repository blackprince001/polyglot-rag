use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::domain::entities::ContentChunk;
use crate::domain::repositories::{ChunkRepository, chunk_repository::ChunkRepositoryError};
use crate::infrastructure::database::models::{ContentChunkModel, NewContentChunkModel};
use crate::infrastructure::database::schema::content_chunks::dsl::*;
use crate::infrastructure::database::{DbPool, get_connection_from_pool};

pub struct PostgresChunkRepository {
    pool: DbPool,
}

impl PostgresChunkRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ChunkRepository for PostgresChunkRepository {
    async fn save(&self, chunk: &ContentChunk) -> Result<Uuid, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let new_chunk = NewContentChunkModel::from(chunk);

        let inserted_chunk: ContentChunkModel = diesel::insert_into(content_chunks)
            .values(&new_chunk)
            .get_result(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_chunk.id)
    }

    async fn save_batch(&self, chunks: &[ContentChunk]) -> Result<Vec<Uuid>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let new_chunks: Vec<NewContentChunkModel> =
            chunks.iter().map(NewContentChunkModel::from).collect();

        let inserted_chunks: Vec<ContentChunkModel> = diesel::insert_into(content_chunks)
            .values(&new_chunks)
            .get_results(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_chunks.into_iter().map(|chunk| chunk.id).collect())
    }

    async fn find_by_id(
        &self,
        chunk_id: Uuid,
    ) -> Result<Option<ContentChunk>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let result = content_chunks
            .find(chunk_id)
            .first::<ContentChunkModel>(&mut conn)
            .optional()
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(result.map(ContentChunk::from))
    }

    // async fn find_by_file_id(
    //     &self,
    //     file_id_param: Uuid,
    // ) -> Result<Vec<ContentChunk>, ChunkRepositoryError> {
    //     let mut conn = get_connection_from_pool(&self.pool)
    //         .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

    //     let models = content_chunks
    //         .filter(file_id.eq(file_id_param))
    //         .order(chunk_index.asc())
    //         .load::<ContentChunkModel>(&mut conn)
    //         .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

    //     Ok(models.into_iter().map(ContentChunk::from).collect())
    // }

    async fn find_by_file_id_paginated(
        &self,
        file_id_param: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<ContentChunk>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let models = content_chunks
            .filter(file_id.eq(file_id_param))
            .order(chunk_index.asc())
            .offset(skip)
            .limit(limit)
            .load::<ContentChunkModel>(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(models.into_iter().map(ContentChunk::from).collect())
    }

    async fn delete(&self, chunk_id: Uuid) -> Result<bool, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(content_chunks.find(chunk_id))
            .execute(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_file_id(&self, file_id_param: Uuid) -> Result<i64, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(content_chunks.filter(file_id.eq(file_id_param)))
            .execute(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count as i64)
    }

    async fn count_by_file_id(&self, file_id_param: Uuid) -> Result<i64, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        content_chunks
            .filter(file_id.eq(file_id_param))
            .count()
            .get_result(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))
    }
}
