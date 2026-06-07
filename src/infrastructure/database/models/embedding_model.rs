use chrono::{DateTime, Utc};
use diesel::prelude::*;
use pgvector::Vector;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::Embedding as DomainEmbedding;
use crate::infrastructure::database::schema::embeddings;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(super::ContentChunkModel, foreign_key = content_chunk_id))]
#[diesel(table_name = embeddings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct EmbeddingModel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub content_chunk_id: Option<Uuid>,
    pub embedding: Option<Vector>,
    pub model_name: String,
    pub model_version: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub generation_parameters: Option<serde_json::Value>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = embeddings)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewEmbeddingModel {
    pub id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub content_chunk_id: Option<Uuid>,
    pub embedding: Option<Vector>,
    pub model_name: String,
    pub model_version: Option<String>,
    pub generated_at: Option<DateTime<Utc>>,
    pub generation_parameters: Option<serde_json::Value>,
}

impl NewEmbeddingModel {
    pub fn for_tenant(tenant_id: Uuid, domain_embedding: &DomainEmbedding) -> Self {
        Self {
            id: None, // Let database generate the ID
            tenant_id,
            content_chunk_id: Some(domain_embedding.content_chunk_id()),
            embedding: Some(domain_embedding.embedding().clone()),
            model_name: domain_embedding.model_name().to_string(),
            model_version: domain_embedding.model_version().map(|s| s.to_string()),
            generated_at: Some(domain_embedding.generated_at()),
            generation_parameters: domain_embedding.generation_parameters().cloned(),
        }
    }
}

impl TryFrom<EmbeddingModel> for DomainEmbedding {
    type Error = String;

    fn try_from(model: EmbeddingModel) -> Result<Self, Self::Error> {
        let embedding_vector = model.embedding.ok_or("Embedding vector is required")?;
        let content_chunk_id = model
            .content_chunk_id
            .ok_or("Content chunk ID is required")?;

        Ok(DomainEmbedding::with_id(
            model.id,
            content_chunk_id,
            model.model_name,
            model.model_version,
            model.generated_at.unwrap_or_else(chrono::Utc::now),
            model.generation_parameters,
            embedding_vector,
        ))
    }
}
