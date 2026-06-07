use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::domain::repositories::auth_repository::AuthRepository;
use crate::infrastructure::auth::key::generate_api_key;
use crate::presentation::http::dto::{
    ApiKeyCreatedDto, ApiKeyListResponseDto, ApiKeySummaryDto, ApiResponse, CreateApiKeyRequest,
    CreateTenantRequest, TenantListResponseDto, TenantResponseDto,
};

pub struct TenantsHandler {
    auth_repository: Arc<dyn AuthRepository>,
}

impl TenantsHandler {
    pub fn new(auth_repository: Arc<dyn AuthRepository>) -> Self {
        Self { auth_repository }
    }

    pub async fn create_tenant(
        State(handler): State<Arc<TenantsHandler>>,
        Json(req): Json<CreateTenantRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let name = req.name.trim();
        if name.is_empty() {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<TenantResponseDto>::error(
                    "VALIDATION_ERROR".to_string(),
                    "Tenant name must not be empty".to_string(),
                    None,
                )),
            ));
        }

        match handler.auth_repository.create_tenant(name).await {
            Ok(tenant) => {
                let dto = TenantResponseDto::from(tenant);
                Ok((StatusCode::CREATED, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<TenantResponseDto>::error(
                    "CREATE_TENANT_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn list_tenants(
        State(handler): State<Arc<TenantsHandler>>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.auth_repository.list_tenants().await {
            Ok(tenants) => {
                let dto = TenantListResponseDto {
                    tenants: tenants.into_iter().map(TenantResponseDto::from).collect(),
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<TenantListResponseDto>::error(
                    "LIST_TENANTS_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn create_api_key(
        State(handler): State<Arc<TenantsHandler>>,
        Path(tenant_id): Path<Uuid>,
        Json(req): Json<CreateApiKeyRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.auth_repository.list_tenants().await {
            Ok(tenants) if !tenants.iter().any(|t| t.id == tenant_id && t.is_active) => {
                return Ok((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<ApiKeyCreatedDto>::error(
                        "TENANT_NOT_FOUND".to_string(),
                        format!("Active tenant with ID {} not found", tenant_id),
                        None,
                    )),
                ));
            }
            Err(e) => {
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ApiKeyCreatedDto>::error(
                        "TENANT_LOOKUP_FAILED".to_string(),
                        e.to_string(),
                        None,
                    )),
                ));
            }
            _ => {}
        }

        let key = generate_api_key();
        let scopes = req.scopes.unwrap_or_default();
        let name = req.name.as_deref();

        match handler
            .auth_repository
            .create_api_key(tenant_id, name, &key.hash, &key.prefix, &scopes)
            .await
        {
            Ok(summary) => {
                let dto = ApiKeyCreatedDto {
                    id: summary.id,
                    tenant_id: summary.tenant_id,
                    name: summary.name,
                    prefix: summary.prefix,
                    scopes: summary.scopes,
                    raw_key: key.raw,
                    key_hash: key.hash,
                    created_at: summary.created_at,
                };
                Ok((StatusCode::CREATED, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<ApiKeyCreatedDto>::error(
                    "CREATE_API_KEY_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn list_api_keys(
        State(handler): State<Arc<TenantsHandler>>,
        Path(tenant_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.auth_repository.list_api_keys(tenant_id).await {
            Ok(keys) => {
                let dto = ApiKeyListResponseDto {
                    keys: keys.into_iter().map(ApiKeySummaryDto::from).collect(),
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<ApiKeyListResponseDto>::error(
                    "LIST_API_KEYS_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn revoke_api_key(
        State(handler): State<Arc<TenantsHandler>>,
        Path((tenant_id, api_key_id)): Path<(Uuid, Uuid)>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .auth_repository
            .revoke_api_key(tenant_id, api_key_id)
            .await
        {
            Ok(true) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success("API key revoked".to_string())),
            )),
            Ok(false) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<String>::error(
                    "API_KEY_NOT_FOUND".to_string(),
                    format!("No active API key {} for tenant {}", api_key_id, tenant_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<String>::error(
                    "REVOKE_API_KEY_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }
}
