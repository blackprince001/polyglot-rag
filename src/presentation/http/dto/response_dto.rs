use std::collections::BTreeMap;

use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn error(code: String, message: String, details: Option<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code,
                message,
                details,
            }),
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn internal_error(context: &str, error: impl std::fmt::Display) -> Self {
        tracing::error!(context, error = %error, "internal error serving request");
        Self::error(
            crate::presentation::http::dto::error_code::ErrorCode::Internal
                .as_str()
                .to_string(),
            "An internal error occurred while processing the request.".to_string(),
            None,
        )
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponseDto {
    pub status: String,
    pub version: String,
    pub dependencies: BTreeMap<String, DependencyStatus>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DependencyStatus {
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponseDto {
    pub message: String,
}
