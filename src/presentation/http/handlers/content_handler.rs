use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

use crate::application::use_cases::{
    ProcessTextDirectUseCase, ProcessUrlDirectUseCase, ProcessYoutubeDirectUseCase,
    process_text_direct::{ProcessTextDirectError, ProcessTextDirectRequest},
    process_url_direct::{ProcessUrlDirectError, ProcessUrlDirectRequest},
    process_youtube_direct::{ProcessYoutubeDirectError, ProcessYoutubeDirectRequest},
};
use crate::presentation::http::dto::error_code::ErrorCode;
use crate::presentation::http::dto::{
    ApiResponse, ContentProcessingResponse, ProcessTextRequest, ProcessUrlRequest,
    ProcessYoutubeRequest,
};
use crate::presentation::http::middleware::TenantContext;

pub struct ContentHandler {
    process_url_use_case: Arc<ProcessUrlDirectUseCase>,
    process_youtube_use_case: Arc<ProcessYoutubeDirectUseCase>,
    process_text_use_case: Arc<ProcessTextDirectUseCase>,
}

impl ContentHandler {
    pub fn new(
        process_url_use_case: Arc<ProcessUrlDirectUseCase>,
        process_youtube_use_case: Arc<ProcessYoutubeDirectUseCase>,
        process_text_use_case: Arc<ProcessTextDirectUseCase>,
    ) -> Self {
        Self {
            process_url_use_case,
            process_youtube_use_case,
            process_text_use_case,
        }
    }

    pub async fn process_url(
        State(handler): State<Arc<ContentHandler>>,
        tenant: TenantContext,
        Json(request_dto): Json<ProcessUrlRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        // Validate URL
        if request_dto.url.trim().is_empty() {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::EmptyUrl.as_str().to_string(),
                    "URL cannot be empty".to_string(),
                    None,
                )),
            ));
        }

        // Convert DTO to use case request
        let use_case_request = ProcessUrlDirectRequest {
            url: request_dto.url,
            filename: request_dto.filename,
            auto_process: request_dto.auto_process.unwrap_or(true),
        };

        // Execute use case
        match handler
            .process_url_use_case
            .execute(tenant.tenant_id, use_case_request)
            .await
        {
            Ok(response) => {
                let dto = ContentProcessingResponse::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok(match e {
                ProcessUrlDirectError::InvalidUrl(msg) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        ErrorCode::InvalidUrl.as_str().to_string(),
                        msg,
                        None,
                    )),
                ),
                ProcessUrlDirectError::ValidationError(msg) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        ErrorCode::ValidationError.as_str().to_string(),
                        msg,
                        None,
                    )),
                ),
                // Repository/queue failures are internal — log full detail, return generic.
                other => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::internal_error("process_url", other)),
                ),
            }),
        }
    }

    pub async fn process_youtube(
        State(handler): State<Arc<ContentHandler>>,
        tenant: TenantContext,
        Json(request_dto): Json<ProcessYoutubeRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        // Validate URL
        if request_dto.url.trim().is_empty() {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::EmptyUrl.as_str().to_string(),
                    "YouTube URL cannot be empty".to_string(),
                    None,
                )),
            ));
        }

        // Basic YouTube URL validation
        if !request_dto.url.contains("youtube.com") && !request_dto.url.contains("youtu.be") {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::InvalidYoutubeUrl.as_str().to_string(),
                    "URL must be a valid YouTube URL".to_string(),
                    None,
                )),
            ));
        }

        // Convert DTO to use case request
        let use_case_request = ProcessYoutubeDirectRequest {
            url: request_dto.url,
            filename: request_dto.filename,
            extract_timestamps: request_dto.extract_timestamps.unwrap_or(true),
            auto_process: request_dto.auto_process.unwrap_or(true),
        };

        // Execute use case
        match handler
            .process_youtube_use_case
            .execute(tenant.tenant_id, use_case_request)
            .await
        {
            Ok(response) => {
                let dto = ContentProcessingResponse::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok(match e {
                ProcessYoutubeDirectError::InvalidUrl(msg) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        ErrorCode::InvalidYoutubeUrl.as_str().to_string(),
                        msg,
                        None,
                    )),
                ),
                ProcessYoutubeDirectError::ValidationError(msg) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        ErrorCode::ValidationError.as_str().to_string(),
                        msg,
                        None,
                    )),
                ),
                other => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::internal_error("process_youtube", other)),
                ),
            }),
        }
    }

    pub async fn process_text(
        State(handler): State<Arc<ContentHandler>>,
        tenant: TenantContext,
        Json(request_dto): Json<ProcessTextRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        if request_dto.text.trim().is_empty() {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::EmptyText.as_str().to_string(),
                    "text cannot be empty".to_string(),
                    None,
                )),
            ));
        }

        let use_case_request = ProcessTextDirectRequest {
            text: request_dto.text,
            filename: request_dto.filename,
            auto_process: request_dto.auto_process.unwrap_or(true),
        };

        match handler
            .process_text_use_case
            .execute(tenant.tenant_id, use_case_request)
            .await
        {
            Ok(response) => {
                let dto = ContentProcessingResponse::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok(match e {
                ProcessTextDirectError::ValidationError(msg) => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::error(
                        ErrorCode::ValidationError.as_str().to_string(),
                        msg,
                        None,
                    )),
                ),
                other => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::internal_error("process_text", other)),
                ),
            }),
        }
    }
}
