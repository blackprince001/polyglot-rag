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
    async fn save_batch(
        &self,
        tenant: Uuid,
        chunks: &[ContentChunk],
    ) -> Result<Vec<Uuid>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let new_chunks: Vec<NewContentChunkModel> = chunks
            .iter()
            .map(|c| NewContentChunkModel::for_tenant(tenant, c))
            .collect();

        let inserted_chunks: Vec<ContentChunkModel> = diesel::insert_into(content_chunks)
            .values(&new_chunks)
            .get_results(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_chunks.into_iter().map(|chunk| chunk.id).collect())
    }

    async fn find_by_id(
        &self,
        tenant: Uuid,
        chunk_id: Uuid,
    ) -> Result<Option<ContentChunk>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let result = content_chunks
            .filter(id.eq(chunk_id))
            .filter(tenant_id.eq(tenant))
            .first::<ContentChunkModel>(&mut conn)
            .optional()
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(result.map(ContentChunk::from))
    }

    async fn find_by_ids(
        &self,
        tenant: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<ContentChunk>, ChunkRepositoryError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let models = content_chunks
            .filter(id.eq_any(ids.to_vec()))
            .filter(tenant_id.eq(tenant))
            .load::<ContentChunkModel>(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(models.into_iter().map(ContentChunk::from).collect())
    }

    async fn find_by_file_id_paginated(
        &self,
        tenant: Uuid,
        file_id_param: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<ContentChunk>, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let models = content_chunks
            .filter(file_id.eq(file_id_param))
            .filter(tenant_id.eq(tenant))
            .order(chunk_index.asc())
            .offset(skip)
            .limit(limit)
            .load::<ContentChunkModel>(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(models.into_iter().map(ContentChunk::from).collect())
    }

    async fn delete(&self, tenant: Uuid, chunk_id: Uuid) -> Result<bool, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(
            content_chunks
                .filter(id.eq(chunk_id))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_file_id(
        &self,
        tenant: Uuid,
        file_id_param: Uuid,
    ) -> Result<i64, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(
            content_chunks
                .filter(file_id.eq(file_id_param))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count as i64)
    }

    async fn count_by_file_id(
        &self,
        tenant: Uuid,
        file_id_param: Uuid,
    ) -> Result<i64, ChunkRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))?;

        content_chunks
            .filter(file_id.eq(file_id_param))
            .filter(tenant_id.eq(tenant))
            .count()
            .get_result(&mut conn)
            .map_err(|e| ChunkRepositoryError::DatabaseError(e.to_string()))
    }
}
