use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::File;
use crate::domain::value_objects::ProcessingStatus;

#[derive(Debug)]
pub enum FileRepositoryError {
    DatabaseError(String),
    ValidationError(String),
}

impl std::fmt::Display for FileRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            FileRepositoryError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for FileRepositoryError {}

#[async_trait]
pub trait FileRepository: Send + Sync {
    async fn save(&self, tenant_id: Uuid, file: &File) -> Result<Uuid, FileRepositoryError>;
    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        id: Uuid,
    ) -> Result<Option<File>, FileRepositoryError>;
    async fn find_by_ids(
        &self,
        tenant_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<File>, FileRepositoryError>;
    async fn find_by_hash(
        &self,
        tenant_id: Uuid,
        hash: &str,
    ) -> Result<Option<File>, FileRepositoryError>;
    async fn find_all(
        &self,
        tenant_id: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<File>, FileRepositoryError>;
    async fn update(&self, tenant_id: Uuid, file: &File) -> Result<(), FileRepositoryError>;
    async fn delete(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, FileRepositoryError>;
    async fn count(&self, tenant_id: Uuid) -> Result<i64, FileRepositoryError>;

    async fn find_stale_for_janitor(
        &self,
        threshold_secs: i64,
        statuses: &[ProcessingStatus],
    ) -> Result<Vec<(File, Uuid)>, FileRepositoryError>;
}
