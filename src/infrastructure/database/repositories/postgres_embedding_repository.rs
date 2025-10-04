use async_trait::async_trait;
use diesel::prelude::*;
use pgvector::Vector;
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

#[async_trait]
impl EmbeddingRepository for PostgresEmbeddingRepository {
    async fn save(&self, embedding_entity: &Embedding) -> Result<Uuid, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let new_embedding = NewEmbeddingModel::from(embedding_entity);

        let inserted_embedding: EmbeddingModel = diesel::insert_into(embeddings)
            .values(&new_embedding)
            .get_result(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_embedding.id)
    }

    async fn save_batch(
        &self,
        embedding_entities: &[Embedding],
    ) -> Result<Vec<Uuid>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let new_embeddings: Vec<NewEmbeddingModel> = embedding_entities
            .iter()
            .map(NewEmbeddingModel::from)
            .collect();

        let inserted_embeddings: Vec<EmbeddingModel> = diesel::insert_into(embeddings)
            .values(&new_embeddings)
            .get_results(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_embeddings.into_iter().map(|emb| emb.id).collect())
    }

    async fn find_by_id(
        &self,
        embedding_id: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let result = embeddings
            .find(embedding_id)
            .select(EmbeddingModel::as_select())
            .first::<EmbeddingModel>(&mut conn)
            .optional()
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_embedding = Embedding::try_from(model)
                    .map_err(|e| EmbeddingRepositoryError::ValidationError(e))?;
                Ok(Some(domain_embedding))
            }
            None => Ok(None),
        }
    }

    async fn find_by_chunk_id(
        &self,
        chunk_id: Uuid,
    ) -> Result<Option<Embedding>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let result = embeddings
            .filter(content_chunk_id.eq(chunk_id))
            .select(EmbeddingModel::as_select())
            .first::<EmbeddingModel>(&mut conn)
            .optional()
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_embedding = Embedding::try_from(model)
                    .map_err(|e| EmbeddingRepositoryError::ValidationError(e))?;
                Ok(Some(domain_embedding))
            }
            None => Ok(None),
        }
    }

    async fn find_by_file_id(
        &self,
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
            .select(EmbeddingModel::as_select())
            .load::<EmbeddingModel>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let mut domain_embeddings = Vec::new();
        for model in models {
            let domain_embedding = Embedding::try_from(model)
                .map_err(|e| EmbeddingRepositoryError::ValidationError(e))?;
            domain_embeddings.push(domain_embedding);
        }

        Ok(domain_embeddings)
    }

    async fn similarity_search(
        &self,
        query_vector: &Vector,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        // This is a simplified version - in a real implementation, you'd use pgvector's similarity functions
        let models = embeddings
            .filter(embedding.is_not_null())
            .limit(limit.into())
            .select(EmbeddingModel::as_select())
            .load::<EmbeddingModel>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            if let (Some(emb_vector), Some(chunk_id)) = (&model.embedding, model.content_chunk_id) {
                // Calculate cosine similarity (simplified)
                let similarity_score = calculate_cosine_similarity(query_vector, emb_vector);

                if let Some(threshold) = similarity_threshold {
                    if similarity_score < threshold {
                        continue;
                    }
                }

                let domain_embedding = Embedding::try_from(model)
                    .map_err(|e| EmbeddingRepositoryError::ValidationError(e))?;

                results.push(SimilaritySearchResult {
                    embedding: domain_embedding,
                    similarity_score,
                    chunk_id,
                });
            }
        }

        // Sort by similarity score (descending)
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        Ok(results)
    }

    async fn similarity_search_by_file(
        &self,
        query_vector: &Vector,
        file_id_param: Uuid,
        limit: i32,
        similarity_threshold: Option<f32>,
    ) -> Result<Vec<SimilaritySearchResult>, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        use crate::infrastructure::database::schema::content_chunks::dsl as chunks_dsl;

        // Join with content_chunks to filter by file_id
        let models = embeddings
            .inner_join(
                chunks_dsl::content_chunks.on(content_chunk_id.eq(chunks_dsl::id.nullable())),
            )
            .filter(chunks_dsl::file_id.eq(file_id_param))
            .filter(embedding.is_not_null())
            .limit(limit.into())
            .select(EmbeddingModel::as_select())
            .load::<EmbeddingModel>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for model in models {
            if let (Some(emb_vector), Some(chunk_id)) = (&model.embedding, model.content_chunk_id) {
                // Calculate cosine similarity
                let similarity_score = calculate_cosine_similarity(query_vector, emb_vector);

                if let Some(threshold) = similarity_threshold {
                    if similarity_score < threshold {
                        continue;
                    }
                }

                let domain_embedding = Embedding::try_from(model)
                    .map_err(|e| EmbeddingRepositoryError::ValidationError(e))?;

                results.push(SimilaritySearchResult {
                    embedding: domain_embedding,
                    similarity_score,
                    chunk_id,
                });
            }
        }

        // Sort by similarity score (descending)
        results.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());

        Ok(results)
    }

    // async fn update(&self, embedding_entity: &Embedding) -> Result<(), EmbeddingRepositoryError> {
    //     let mut conn = get_connection_from_pool(&self.pool)
    //         .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

    //     let update_model = NewEmbeddingModel::from(embedding_entity);

    //     diesel::update(embeddings.find(embedding_entity.id()))
    //         .set(&update_model)
    //         .execute(&mut conn)
    //         .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

    //     Ok(())
    // }

    async fn delete(&self, embedding_id: Uuid) -> Result<bool, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(embeddings.find(embedding_id))
            .execute(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_chunk_id(&self, chunk_id: Uuid) -> Result<bool, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count = diesel::delete(embeddings.filter(content_chunk_id.eq(chunk_id)))
            .execute(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn delete_by_file_id(
        &self,
        file_id_param: Uuid,
    ) -> Result<i64, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        use crate::infrastructure::database::schema::content_chunks::dsl as chunks_dsl;

        // Use a subquery to find embeddings that belong to chunks of the specified file
        let chunk_ids: Vec<Uuid> = chunks_dsl::content_chunks
            .filter(chunks_dsl::file_id.eq(file_id_param))
            .select(chunks_dsl::id)
            .load::<Uuid>(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        if chunk_ids.is_empty() {
            return Ok(0);
        }

        // Delete embeddings that belong to those chunks
        let deleted_count = diesel::delete(embeddings.filter(content_chunk_id.eq_any(chunk_ids)))
            .execute(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count as i64)
    }

    async fn count(&self) -> Result<i64, EmbeddingRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

        embeddings
            .count()
            .get_result(&mut conn)
            .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))
    }

    // async fn count_by_model(
    //     &self,
    //     model_name_param: &str,
    // ) -> Result<i64, EmbeddingRepositoryError> {
    //     let mut conn = get_connection_from_pool(&self.pool)
    //         .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))?;

    //     embeddings
    //         .filter(model_name.eq(model_name_param))
    //         .count()
    //         .get_result(&mut conn)
    //         .map_err(|e| EmbeddingRepositoryError::DatabaseError(e.to_string()))
    // }
}

// Helper function to calculate cosine similarity
fn calculate_cosine_similarity(a: &Vector, b: &Vector) -> f32 {
    let a_slice = a.as_slice();
    let b_slice = b.as_slice();

    if a_slice.len() != b_slice.len() {
        return 0.0;
    }

    let dot_product: f32 = a_slice.iter().zip(b_slice.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a_slice.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b_slice.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}
