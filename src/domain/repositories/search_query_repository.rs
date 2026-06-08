use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::entities::SearchQuery;

#[derive(Debug)]
pub enum SearchQueryRepositoryError {
    DatabaseError(String),
}

impl std::fmt::Display for SearchQueryRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchQueryRepositoryError::DatabaseError(msg) => {
                write!(f, "Database error: {}", msg)
            }
        }
    }
}

impl std::error::Error for SearchQueryRepositoryError {}

#[async_trait]
pub trait SearchQueryRepository: Send + Sync {
    async fn save(&self, query: &SearchQuery) -> Result<(), SearchQueryRepositoryError>;
    async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<SearchQuery>, SearchQueryRepositoryError>;
}
