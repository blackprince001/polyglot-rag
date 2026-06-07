use std::sync::Arc;
use uuid::Uuid;

use crate::domain::entities::{Asset, ContentChunk, File};
use crate::domain::repositories::{
    AssetRepository, ChunkRepository, FileRepository, asset_repository::AssetRepositoryError,
    chunk_repository::ChunkRepositoryError, file_repository::FileRepositoryError,
};

#[derive(Debug)]
pub enum GetFileChunksError {
    FileNotFound(Uuid),
    RepositoryError(String),
}

impl std::fmt::Display for GetFileChunksError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetFileChunksError::FileNotFound(id) => write!(f, "File not found: {}", id),
            GetFileChunksError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for GetFileChunksError {}

impl From<FileRepositoryError> for GetFileChunksError {
    fn from(error: FileRepositoryError) -> Self {
        GetFileChunksError::RepositoryError(error.to_string())
    }
}

impl From<ChunkRepositoryError> for GetFileChunksError {
    fn from(error: ChunkRepositoryError) -> Self {
        GetFileChunksError::RepositoryError(error.to_string())
    }
}

impl From<AssetRepositoryError> for GetFileChunksError {
    fn from(error: AssetRepositoryError) -> Self {
        GetFileChunksError::RepositoryError(error.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct GetFileChunksRequest {
    pub file_id: Uuid,
    pub skip: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct GetFileChunksResponse {
    pub file: File,
    pub chunks: Vec<ContentChunk>,
    pub assets: Vec<Asset>,
}

pub struct GetFileChunksUseCase {
    file_repository: Arc<dyn FileRepository>,
    chunk_repository: Arc<dyn ChunkRepository>,
    asset_repository: Arc<dyn AssetRepository>,
}

impl GetFileChunksUseCase {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        chunk_repository: Arc<dyn ChunkRepository>,
        asset_repository: Arc<dyn AssetRepository>,
    ) -> Self {
        Self {
            file_repository,
            chunk_repository,
            asset_repository,
        }
    }

    pub async fn execute(
        &self,
        tenant_id: Uuid,
        request: GetFileChunksRequest,
    ) -> Result<GetFileChunksResponse, GetFileChunksError> {
        // Verify file exists (and capture its metadata for the response)
        let file = self
            .file_repository
            .find_by_id(tenant_id, request.file_id)
            .await?
            .ok_or(GetFileChunksError::FileNotFound(request.file_id))?;

        let skip = request.skip.unwrap_or(0);
        let limit = request.limit.unwrap_or(50).min(100); // Cap at 100 chunks per request

        // Get chunks for the file
        let chunks = self
            .chunk_repository
            .find_by_file_id_paginated(tenant_id, request.file_id, skip, limit)
            .await?;

        // Load any extracted assets (images, etc) for this file.
        let assets = self
            .asset_repository
            .find_by_file_id(tenant_id, request.file_id)
            .await?;

        Ok(GetFileChunksResponse {
            file,
            chunks,
            assets,
        })
    }
}
