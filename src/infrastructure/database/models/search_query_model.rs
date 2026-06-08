use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde_json::Value;
use uuid::Uuid;

use crate::domain::entities::SearchQuery as DomainSearchQuery;
use crate::infrastructure::database::schema::search_queries;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = search_queries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct SearchQueryModel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub query_text: String,
    pub results_count: i32,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<String>,
    pub search_parameters: Option<Value>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = search_queries)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewSearchQueryModel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub query_text: String,
    pub results_count: i32,
    pub created_at: DateTime<Utc>,
    pub user_id: Option<String>,
    pub search_parameters: Option<Value>,
}

impl From<NewSearchQueryModel> for DomainSearchQuery {
    fn from(m: NewSearchQueryModel) -> Self {
        DomainSearchQuery::with_id(
            m.id,
            m.tenant_id,
            m.query_text,
            m.results_count,
            m.created_at,
            m.user_id,
            m.search_parameters,
        )
    }
}

impl From<SearchQueryModel> for DomainSearchQuery {
    fn from(m: SearchQueryModel) -> Self {
        DomainSearchQuery::with_id(
            m.id,
            m.tenant_id,
            m.query_text,
            m.results_count,
            m.created_at,
            m.user_id,
            m.search_parameters,
        )
    }
}

impl From<&DomainSearchQuery> for NewSearchQueryModel {
    fn from(q: &DomainSearchQuery) -> Self {
        Self {
            id: q.id(),
            tenant_id: q.tenant_id(),
            query_text: q.query_text().to_string(),
            results_count: q.results_count(),
            created_at: q.created_at(),
            user_id: q.user_id().map(|s| s.to_string()),
            search_parameters: q.search_parameters().cloned(),
        }
    }
}
