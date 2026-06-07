use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::repositories::auth_repository::{ApiKeySummary, TenantSummary};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: Option<String>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResponseDto {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<TenantSummary> for TenantResponseDto {
    fn from(t: TenantSummary) -> Self {
        Self {
            id: t.id,
            name: t.name,
            is_active: t.is_active,
            created_at: t.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantListResponseDto {
    pub tenants: Vec<TenantResponseDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeySummaryDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub prefix: String,
    pub scopes: Vec<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<ApiKeySummary> for ApiKeySummaryDto {
    fn from(k: ApiKeySummary) -> Self {
        Self {
            id: k.id,
            tenant_id: k.tenant_id,
            name: k.name,
            prefix: k.prefix,
            scopes: k.scopes,
            last_used_at: k.last_used_at,
            revoked_at: k.revoked_at,
            created_at: k.created_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyListResponseDto {
    pub keys: Vec<ApiKeySummaryDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyCreatedDto {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub prefix: String,
    pub scopes: Vec<String>,

    pub raw_key: String,
    pub key_hash: String,
    pub created_at: DateTime<Utc>,
}
