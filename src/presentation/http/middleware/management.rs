use std::sync::Arc;

use axum::{
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};

use crate::presentation::http::dto::ApiResponse;
use crate::presentation::http::dto::error_code::ErrorCode;

#[derive(Clone)]
pub struct ManagementKeyConfig {
    pub key: Arc<Option<String>>,
}

impl ManagementKeyConfig {
    pub fn from_env() -> Self {
        let key = std::env::var("TENANT_MANAGEMENT_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        match &key {
            Some(k) => tracing::info!(
                "Tenant management key loaded (length={}); management routes are live",
                k.len()
            ),
            None => tracing::warn!(
                "TENANT_MANAGEMENT_KEY is not set; tenant management routes will return 503. \
                 Set it to enable POST/GET /tenants and key management."
            ),
        }
        Self { key: Arc::new(key) }
    }
}

fn service_unavailable(message: &str) -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(ApiResponse::<()>::error(
            ErrorCode::ManagementDisabled.as_str().to_string(),
            message.to_string(),
            None,
        )),
    )
        .into_response()
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(ApiResponse::<()>::error(
            ErrorCode::Unauthorized.as_str().to_string(),
            message.to_string(),
            None,
        )),
    )
        .into_response()
}

fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }
    if let Some(value) = parts.headers.get("x-management-key") {
        if let Ok(s) = value.to_str() {
            return Some(s.trim().to_string());
        }
    }
    None
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

pub async fn require_management_key(
    axum::extract::State(cfg): axum::extract::State<ManagementKeyConfig>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let expected = match cfg.key.as_ref() {
        Some(k) => k,
        None => {
            return service_unavailable(
                "Tenant management is disabled: set TENANT_MANAGEMENT_KEY to enable it",
            );
        }
    };

    let (parts, body) = req.into_parts();

    let presented = match extract_token(&parts) {
        Some(t) if !t.is_empty() => t,
        _ => {
            return unauthorized(
                "Missing management key (use 'Authorization: Bearer <key>' or 'X-Management-Key')",
            );
        }
    };

    if !ct_eq(presented.as_bytes(), expected.as_bytes()) {
        return unauthorized("Invalid management key");
    }

    let req = axum::extract::Request::from_parts(parts, body);
    next.run(req).await
}
