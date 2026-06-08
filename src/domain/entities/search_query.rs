use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct SearchQuery {
    id: Uuid,
    tenant_id: Uuid,
    query_text: String,
    results_count: i32,
    created_at: DateTime<Utc>,
    user_id: Option<String>,
    search_parameters: Option<Value>,
}

impl SearchQuery {
    pub fn new(
        tenant_id: Uuid,
        query_text: String,
        results_count: i32,
        user_id: Option<String>,
        search_parameters: Option<Value>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            tenant_id,
            query_text,
            results_count,
            created_at: Utc::now(),
            user_id,
            search_parameters,
        }
    }

    pub fn with_id(
        id: Uuid,
        tenant_id: Uuid,
        query_text: String,
        results_count: i32,
        created_at: DateTime<Utc>,
        user_id: Option<String>,
        search_parameters: Option<Value>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            query_text,
            results_count,
            created_at,
            user_id,
            search_parameters,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn query_text(&self) -> &str {
        &self.query_text
    }

    pub fn results_count(&self) -> i32 {
        self.results_count
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub fn search_parameters(&self) -> Option<&Value> {
        self.search_parameters.as_ref()
    }
}
