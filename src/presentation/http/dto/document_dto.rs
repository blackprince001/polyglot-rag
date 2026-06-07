use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::use_cases::search_content::DocumentMatch;
use crate::domain::entities::{Asset, ContentChunk, File};
use crate::presentation::http::dto::asset_dto::AssetDto;

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentWithChunksDto {
    pub id: Uuid,
    pub file_name: String,
    pub file_path: String,
    pub file_type: Option<String>,
    pub processing_status: String,
    pub chunks: Vec<DocumentChunkDto>,

    #[serde(default)]
    pub assets: Vec<AssetDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentChunkDto {
    pub chunk_id: Uuid,
    pub chunk_text: String,
    pub chunk_index: i32,
    pub page_number: Option<i32>,
    pub section_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity_score: Option<f32>,
}

impl DocumentChunkDto {
    fn from_chunk(chunk: &ContentChunk, similarity_score: Option<f32>) -> Self {
        Self {
            chunk_id: chunk.id(),
            chunk_text: chunk.chunk_text().to_string(),
            chunk_index: chunk.chunk_index(),
            page_number: chunk.page_number(),
            section_path: chunk.section_path().map(|s| s.to_string()),
            similarity_score,
        }
    }
}

impl DocumentWithChunksDto {
    fn header(file: &File) -> Self {
        Self {
            id: file.id(),
            file_name: file.file_name().to_string(),
            file_path: file.file_path().to_string(),
            file_type: file.file_type().map(|s| s.to_string()),
            processing_status: file.processing_status().to_string(),
            chunks: Vec::new(),
            assets: Vec::new(),
        }
    }

    /// Build from a single document and its chunks (no similarity scores).
    pub fn from_file_and_chunks(file: &File, chunks: &[ContentChunk]) -> Self {
        let mut dto = Self::header(file);
        dto.chunks = chunks
            .iter()
            .map(|c| DocumentChunkDto::from_chunk(c, None))
            .collect();
        dto
    }

    /// Attach the document's assets to an already-built DTO.
    pub fn with_assets(mut self, assets: &[Asset]) -> Self {
        self.assets = assets.iter().map(AssetDto::from_asset).collect();
        self
    }
}

impl From<DocumentMatch> for DocumentWithChunksDto {
    fn from(m: DocumentMatch) -> Self {
        let mut dto = DocumentWithChunksDto::header(&m.file);
        dto.chunks = m
            .chunks
            .iter()
            .map(|sc| DocumentChunkDto::from_chunk(&sc.chunk, Some(sc.similarity_score)))
            .collect();
        dto.with_assets(&m.assets)
    }
}
