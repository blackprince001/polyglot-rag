use async_trait::async_trait;
use diesel::prelude::*;

use crate::domain::entities::SearchQuery;
use crate::domain::repositories::SearchQueryRepository;
use crate::domain::repositories::search_query_repository::SearchQueryRepositoryError;
use crate::infrastructure::database::models::NewSearchQueryModel;
use crate::infrastructure::database::schema::search_queries::dsl::*;
use crate::infrastructure::database::{DbPool, get_connection_from_pool};

pub struct PostgresSearchQueryRepository {
    pool: DbPool,
}

impl PostgresSearchQueryRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchQueryRepository for PostgresSearchQueryRepository {
    async fn save(&self, query: &SearchQuery) -> Result<(), SearchQueryRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| SearchQueryRepositoryError::DatabaseError(e.to_string()))?;

        let new_row = NewSearchQueryModel::from(query);

        diesel::insert_into(search_queries)
            .values(&new_row)
            .execute(&mut conn)
            .map_err(|e| SearchQueryRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}
