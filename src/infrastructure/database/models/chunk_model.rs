use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::ContentChunk as DomainChunk;
use crate::infrastructure::database::schema::content_chunks;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(super::FileModel, foreign_key = file_id))]
#[diesel(table_name = content_chunks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ContentChunkModel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub file_id: Uuid,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub token_count: Option<i32>,
    pub page_number: Option<i32>,
    pub section_path: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Insertable, AsChangeset)]
#[diesel(table_name = content_chunks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewContentChunkModel {
    pub id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub file_id: Uuid,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub token_count: Option<i32>,
    pub page_number: Option<i32>,
    pub section_path: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

impl NewContentChunkModel {
    pub fn for_tenant(tenant_id: Uuid, domain_chunk: &DomainChunk) -> Self {
        Self {
            id: None, // Let database generate the ID
            tenant_id,
            file_id: domain_chunk.file_id(),
            chunk_text: domain_chunk.chunk_text().to_string(),
            chunk_index: domain_chunk.chunk_index(),
            token_count: domain_chunk.token_count(),
            page_number: domain_chunk.page_number(),
            section_path: domain_chunk.section_path().map(|s| s.to_string()),
            created_at: Some(domain_chunk.created_at()),
        }
    }
}

impl From<ContentChunkModel> for DomainChunk {
    fn from(model: ContentChunkModel) -> Self {
        DomainChunk::with_id(
            model.id,
            model.file_id,
            model.chunk_text,
            model.chunk_index,
            model.token_count,
            model.page_number,
            model.section_path,
            model.created_at.unwrap_or_else(chrono::Utc::now),
        )
    }
}
