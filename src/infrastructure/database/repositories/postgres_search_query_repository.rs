use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::domain::entities::SearchQuery;
use crate::domain::repositories::SearchQueryRepository;
use crate::domain::repositories::search_query_repository::SearchQueryRepositoryError;
use crate::infrastructure::database::models::{NewSearchQueryModel, SearchQueryModel};
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

    async fn list_by_tenant(
        &self,
        tid: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<SearchQuery>, SearchQueryRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| SearchQueryRepositoryError::DatabaseError(e.to_string()))?;

        let rows = search_queries
            .filter(tenant_id.eq(tid))
            .order(created_at.desc())
            .offset(skip)
            .limit(limit)
            .load::<SearchQueryModel>(&mut conn)
            .map_err(|e| SearchQueryRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(SearchQuery::from).collect())
    }
}
