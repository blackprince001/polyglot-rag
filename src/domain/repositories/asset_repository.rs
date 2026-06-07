use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::Asset;

#[derive(Debug)]
pub enum AssetRepositoryError {
    DatabaseError(String),
}

impl std::fmt::Display for AssetRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for AssetRepositoryError {}

#[async_trait]
pub trait AssetRepository: Send + Sync {
    async fn save_batch(
        &self,
        tenant_id: Uuid,
        assets: &[Asset],
    ) -> Result<Vec<Uuid>, AssetRepositoryError>;

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<Asset>, AssetRepositoryError>;

    async fn find_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<Asset>, AssetRepositoryError>;

    async fn delete_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<i64, AssetRepositoryError>;
}
