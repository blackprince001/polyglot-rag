use async_trait::async_trait;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Double, Uuid as SqlUuid};
use pgvector::Vector;
use pgvector::sql_types::Vector as SqlVector;
use uuid::Uuid;

use crate::domain::entities::Embedding;
use crate::domain::repositories::{
    EmbeddingRepository,
    embedding_repository::{EmbeddingRepositoryError, SimilaritySearchResult},
};
use crate::infrastructure::database::models::{EmbeddingModel, NewEmbeddingModel};
use crate::infrastructure::database::schema::embeddings::dsl::*;
use crate::infrastructure::database::{DbPool, get_connection_from_pool};

pub struct PostgresEmbeddingRepository {
    pool: DbPool,
}

impl PostgresEmbeddingRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

/// Row shape returned by the raw pgvector similarity queries.
#[derive(QueryableByName)]
struct SimRow {
    #[diesel(sql_type = SqlUuid)]
    chunk_id: Uuid,
    #[diesel(sql_type = Double)]
    similarity: f64,
}

#[async_trait]
impl EmbeddingRepository for PostgresEmbeddingRepository {
    async fn save_batch(
        &self,
        tenant: Uuid,
        embedding_entities: &[Embedding],
    ) -> Result<Vec<Uuid>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let new_embeddings: Vec<NewEmbeddingModel> = embedding_entities
            .iter()
            .map(|e| NewEmbeddingModel::for_tenant(tenant, e))
            .collect();

        let inserted_embeddings: Vec<EmbeddingModel> = diesel::insert_into(embeddings)
            .values(&new_embeddings)
            .get_results(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_embeddings.into_iter().map(|emb| emb.id).collect())
    }

    async fn find_by_id(
        &self,
        tenant: Uuid,
        embedding_id: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let result = embeddings
            .filter(id.eq(embedding_id))
            .filter(tenant_id.eq(tenant))
            .select(EmbeddingModel::as_select())
            .first::<EmbeddingModel>(&mut conn)
            .optional()
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_embedding = Embedding::try_from(model)
                    .map_err(EmbeddingRepositoryError::ValidationError)?;
                Ok(Some(domain_embedding))
            }
            None => Ok(None),
        }
    }

    async fn find_by_chunk_id(
        &self,
        tenant: Uuid,
        chunk_id_param: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let result = embeddings
            .filter(content_chunk_id.eq(chunk_id_param))
            .filter(tenant_id.eq(tenant))
            .select(EmbeddingModel::as_select())
            .first::<EmbeddingModel>(&mut conn)
            .optional()
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_embedding = Embedding::try_from(model)
                    .map_err(EmbeddingRepositoryError::ValidationError)?;
                Ok(Some(domain_embedding))
            }
            None => Ok(None),
        }
    }

    async fn find_by_file_id(
        &self,
        tenant: Uuid,
        file_id_param: Uuid,
    ) -> Result<Vec<Embedding>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        use crate::infrastructure::database::schema::content_chunks::dsl as chunks_dsl;

        let models = embeddings
            .inner_join(
                chunks_dsl::content_chunks.on(content_chunk_id.eq(chunks_dsl::id.nullable())),
            )
            .filter(chunks_dsl::file_id.eq(file_id_param))
            .filter(tenant_id.eq(tenant))
            .select(EmbeddingModel::as_select())
            .load::<EmbeddingModel>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let mut domain_embeddings = Vec::new();
        for model in models {
            let domain_embedding =
                Embedding::try_from(model).map_err(EmbeddingRepositoryError::ValidationError)?;
            domain_embeddings.push(domain_embedding);
        }

        Ok(domain_embeddings)
    }

    async fn similarity_search(
        &self,
        tenant: Uuid,
        query_vector: &Vector,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        // Cosine similarity = 1 - cosine distance (`<=>`). No threshold => -1.0
        // passes everything (cosine similarity is bounded to [-1, 1]).
        let threshold = similarity_threshold.unwrap_or(-1.0) as f64;

        let sql = "SELECT content_chunk_id AS chunk_id, \
                          (1 - (embedding <=> $1)) AS similarity \
                   FROM embeddings \
                   WHERE tenant_id = $3 \
                     AND embedding IS NOT NULL \
                     AND content_chunk_id IS NOT NULL \
                     AND (1 - (embedding <=> $1)) >= $2 \
                   ORDER BY embedding <=> $1 \
                   LIMIT $4";

        let rows = diesel::sql_query(sql)
            .bind::<SqlVector, _>(query_vector.clone())
            .bind::<Double, _>(threshold)
            .bind::<SqlUuid, _>(tenant)
            .bind::<BigInt, _>(limit as i64)
            .load::<SimRow>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| SimilaritySearchResult {
                chunk_id: r.chunk_id,
                similarity_score: r.similarity as f32,
            })
            .collect())
    }

    async fn similarity_search_by_file(
        &self,
        tenant: Uuid,
        query_vector: &Vector,
        file_id_param: Uuid,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let threshold = similarity_threshold.unwrap_or(-1.0) as f64;

        let sql = "SELECT e.content_chunk_id AS chunk_id, \
                          (1 - (e.embedding <=> $1)) AS similarity \
                   FROM embeddings e \
                   JOIN content_chunks c ON c.id = e.content_chunk_id \
                   WHERE e.tenant_id = $3 \
                     AND c.file_id = $5 \
                     AND e.embedding IS NOT NULL \
                     AND (1 - (e.embedding <=> $1)) >= $2 \
                   ORDER BY e.embedding <=> $1 \
                   LIMIT $4";

        let rows = diesel::sql_query(sql)
            .bind::<SqlVector, _>(query_vector.clone())
            .bind::<Double, _>(threshold)
            .bind::<SqlUuid, _>(tenant)
            .bind::<BigInt, _>(limit as i64)
            .bind::<SqlUuid, _>(file_id_param)
            .load::<SimRow>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|r| SimilaritySearchResult {
                chunk_id: r.chunk_id,
                similarity_score: r.similarity as f32,
            })
            .collect())
    }

    async fn delete(
        &self,
        tenant: Uuid,
        embedding_id: Uuid,
    ) -> Result<bool, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(
            embeddings
                .filter(id.eq(embedding_id))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_chunk_id(
        &self,
        tenant: Uuid,
        chunk_id_param: Uuid,
    ) -> Result<bool, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(
            embeddings
                .filter(content_chunk_id.eq(chunk_id_param))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_file_id(
        &self,
        tenant: Uuid,
        file_id_param: Uuid,
    ) -> Result<i64, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        use crate::infrastructure::database::schema::content_chunks::dsl as chunks_dsl;

        let chunk_ids: Vec<Uuid> = chunks_dsl::content_chunks
            .filter(chunks_dsl::file_id.eq(file_id_param))
            .filter(chunks_dsl::tenant_id.eq(tenant))
            .select(chunks_dsl::id)
            .load::<Uuid>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        if chunk_ids.is_empty() {
            return Ok(0);
        }

        let deleted_count = diesel::delete(
            embeddings
                .filter(content_chunk_id.eq_any(chunk_ids))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count as i64)
    }

    async fn count(&self, tenant: Uuid) -> Result<i64, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        embeddings
            .filter(tenant_id.eq(tenant))
            .count()
            .get_result(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))
    }
}
