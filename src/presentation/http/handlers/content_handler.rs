use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use std::sync::Arc;

use crate::application::use_cases::{
    ProcessTextDirectUseCase, ProcessUrlDirectUseCase, ProcessYoutubeDirectUseCase,
    process_text_direct::{ProcessTextDirectError, ProcessTextDirectRequest},
    process_url_direct::{ProcessUrlDirectError, ProcessUrlDirectRequest},
    process_youtube_direct::{ProcessYoutubeDirectError, ProcessYoutubeDirectRequest},
};
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
                    "EMPTY_URL".to_string(),
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
            Err(e) => {
                let (status, error_code) = match e {
                    ProcessUrlDirectError::InvalidUrl(_) => {
                        (StatusCode::BAD_REQUEST, "INVALID_URL")
                    }
                    ProcessUrlDirectError::ValidationError(_) => {
                        (StatusCode::BAD_REQUEST, "VALIDATION_ERROR")
                    }
                    ProcessUrlDirectError::RepositoryError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "REPOSITORY_ERROR")
                    }
                    ProcessUrlDirectError::QueueError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "QUEUE_ERROR")
                    }
                };

                Ok((
                    status,
                    Json(ApiResponse::error(
                        error_code.to_string(),
                        e.to_string(),
                        None,
                    )),
                ))
            }
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
                    "EMPTY_URL".to_string(),
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
                    "INVALID_YOUTUBE_URL".to_string(),
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
            language_preference: request_dto
                .language_preference
                .unwrap_or_else(|| vec!["en".to_string()]),
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
            Err(e) => {
                let (status, error_code) = match e {
                    ProcessYoutubeDirectError::InvalidUrl(_) => {
                        (StatusCode::BAD_REQUEST, "INVALID_YOUTUBE_URL")
                    }
                    ProcessYoutubeDirectError::ValidationError(_) => {
                        (StatusCode::BAD_REQUEST, "VALIDATION_ERROR")
                    }
                    ProcessYoutubeDirectError::RepositoryError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "REPOSITORY_ERROR")
                    }
                    ProcessYoutubeDirectError::QueueError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "QUEUE_ERROR")
                    }
                };

                Ok((
                    status,
                    Json(ApiResponse::error(
                        error_code.to_string(),
                        e.to_string(),
                        None,
                    )),
                ))
            }
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
                    "EMPTY_TEXT".to_string(),
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
            Err(e) => {
                let (status, error_code) = match e {
                    ProcessTextDirectError::ValidationError(_) => {
                        (StatusCode::BAD_REQUEST, "VALIDATION_ERROR")
                    }
                    ProcessTextDirectError::StorageError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR")
                    }
                    ProcessTextDirectError::RepositoryError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "REPOSITORY_ERROR")
                    }
                    ProcessTextDirectError::QueueError(_) => {
                        (StatusCode::INTERNAL_SERVER_ERROR, "QUEUE_ERROR")
                    }
                };

                Ok((
                    status,
                    Json(ApiResponse::error(
                        error_code.to_string(),
                        e.to_string(),
                        None,
                    )),
                ))
            }
        }
    }
}
