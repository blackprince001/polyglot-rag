use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug)]
pub enum AuthRepositoryError {
    DatabaseError(String),
}

impl std::fmt::Display for AuthRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthRepositoryError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

impl std::error::Error for AuthRepositoryError {}

#[derive(Debug, Clone)]
pub struct AuthIdentity {
    pub tenant_id: Uuid,
    pub api_key_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct TenantSummary {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ApiKeySummary {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait AuthRepository: Send + Sync {
    async fn resolve_key(
        &self,
        key_hash: &str,
    ) -> Result<Option<AuthIdentity>, AuthRepositoryError>;

    async fn touch_key(&self, api_key_id: Uuid) -> Result<(), AuthRepositoryError>;

    async fn create_tenant(&self, name: &str) -> Result<TenantSummary, AuthRepositoryError>;

    async fn create_api_key(
        &self,
        tenant_id: Uuid,
        name: Option<&str>,
        key_hash: &str,
        key_prefix: &str,
        scopes: &[String],
    ) -> Result<ApiKeySummary, AuthRepositoryError>;

    async fn list_tenants(&self) -> Result<Vec<TenantSummary>, AuthRepositoryError>;

    async fn list_api_keys(
        &self,
        tenant_id: Uuid,
    ) -> Result<Vec<ApiKeySummary>, AuthRepositoryError>;

    async fn revoke_api_key(
        &self,
        tenant_id: Uuid,
        api_key_id: Uuid,
    ) -> Result<bool, AuthRepositoryError>;
}
