use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, State},
    http::{StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use uuid::Uuid;

use crate::domain::repositories::AuthRepository;
use crate::infrastructure::auth::key::hash_key;
use crate::presentation::http::dto::ApiResponse;
use crate::presentation::http::dto::error_code::ErrorCode;

#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
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

fn extract_key(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(token) = s.strip_prefix("Bearer ") {
                return Some(token.trim().to_string());
            }
        }
    }
    if let Some(value) = parts.headers.get("x-api-key") {
        if let Ok(s) = value.to_str() {
            return Some(s.trim().to_string());
        }
    }
    None
}

pub async fn require_api_key(
    State(auth): State<Arc<dyn AuthRepository>>,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let (mut parts, body) = req.into_parts();

    let raw_key = match extract_key(&parts) {
        Some(k) if !k.is_empty() => k,
        _ => {
            return unauthorized(
                "Missing API key (use 'Authorization: Bearer <key>' or 'X-API-Key')",
            );
        }
    };

    let hash = hash_key(&raw_key);

    let identity = match auth.resolve_key(&hash).await {
        Ok(Some(identity)) => identity,
        Ok(None) => return unauthorized("Invalid or revoked API key"),
        Err(e) => {
            tracing::error!("Auth lookup failed: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(ApiResponse::<()>::error(
                    ErrorCode::AuthError.as_str().to_string(),
                    "Authentication backend error".to_string(),
                    None,
                )),
            )
                .into_response();
        }
    };

    {
        let auth = auth.clone();
        let key_id = identity.api_key_id;
        tokio::spawn(async move {
            let _ = auth.touch_key(key_id).await;
        });
    }

    parts.extensions.insert(TenantContext {
        tenant_id: identity.tenant_id,
    });

    let req = axum::extract::Request::from_parts(parts, body);
    next.run(req).await
}

impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or_else(|| unauthorized("Missing tenant context"))
    }
}
