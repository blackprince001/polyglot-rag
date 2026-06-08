use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::use_cases::{
    CancelJobUseCase, GetJobStatusUseCase, QueueProcessingJobUseCase,
    cancel_job::{CancelJobError, CancelJobRequest},
    get_job_status::{GetJobStatusError, GetJobStatusRequest},
    queue_processing_job::QueueJobError,
};
use crate::presentation::http::dto::error_code::ErrorCode;
use crate::presentation::http::dto::{
    ApiResponse, CancelJobResponseDto, JobStatusDto, ProcessUrlRequestDto,
    ProcessYoutubeRequestDto, QueueJobResponseDto,
};
use crate::presentation::http::middleware::TenantContext;

pub struct JobHandler {
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
    get_job_status_use_case: Arc<GetJobStatusUseCase>,
    cancel_job_use_case: Arc<CancelJobUseCase>,
}

impl JobHandler {
    pub fn new(
        queue_job_use_case: Arc<QueueProcessingJobUseCase>,
        get_job_status_use_case: Arc<GetJobStatusUseCase>,
        cancel_job_use_case: Arc<CancelJobUseCase>,
    ) -> Self {
        Self {
            queue_job_use_case,
            get_job_status_use_case,
            cancel_job_use_case,
        }
    }

    // Queue file processing job
    pub async fn queue_file_processing(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .queue_job_use_case
            .queue_file_processing(tenant.tenant_id, file_id)
            .await
        {
            Ok(response) => {
                let dto = QueueJobResponseDto::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(QueueJobError::ValidationError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::QueueValidationFailed.as_str().to_string(),
                    msg,
                    None,
                )),
            )),
            Err(QueueJobError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::FileNotFound.as_str().to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("queue_failed", e)),
            )),
        }
    }

    // Queue URL extraction job
    pub async fn queue_url_extraction(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
        Json(request): Json<ProcessUrlRequestDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .queue_job_use_case
            .queue_url_extraction(tenant.tenant_id, file_id, request.url)
            .await
        {
            Ok(response) => {
                let dto = QueueJobResponseDto::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(QueueJobError::ValidationError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::QueueValidationFailed.as_str().to_string(),
                    msg,
                    None,
                )),
            )),
            Err(QueueJobError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::FileNotFound.as_str().to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("queue_failed", e)),
            )),
        }
    }

    // Queue YouTube extraction job
    pub async fn queue_youtube_extraction(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
        Json(request): Json<ProcessYoutubeRequestDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .queue_job_use_case
            .queue_youtube_extraction(tenant.tenant_id, file_id, request.url)
            .await
        {
            Ok(response) => {
                let dto = QueueJobResponseDto::from(response);
                Ok((StatusCode::ACCEPTED, Json(ApiResponse::success(dto))))
            }
            Err(QueueJobError::ValidationError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    ErrorCode::QueueValidationFailed.as_str().to_string(),
                    msg,
                    None,
                )),
            )),
            Err(QueueJobError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::FileNotFound.as_str().to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("queue_failed", e)),
            )),
        }
    }

    // Get job status
    pub async fn get_job_status(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(job_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = GetJobStatusRequest { job_id };

        match handler
            .get_job_status_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = JobStatusDto::from(response);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(GetJobStatusError::JobNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::JobNotFound.as_str().to_string(),
                    format!("Job with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("job_lookup_failed", e)),
            )),
        }
    }

    // Get jobs for a specific file
    pub async fn get_file_jobs(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler
            .get_job_status_use_case
            .get_jobs_for_file(tenant.tenant_id, file_id)
            .await
        {
            Ok(jobs) => {
                let dtos: Vec<JobStatusDto> =
                    jobs.into_iter().map(JobStatusDto::from_job).collect();
                Ok((StatusCode::OK, Json(ApiResponse::success(dtos))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("fetch_failed", e)),
            )),
        }
    }

    // Get all active jobs
    pub async fn get_active_jobs(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.get_job_status_use_case.get_active_jobs().await {
            Ok(jobs) => {
                let dtos: Vec<JobStatusDto> = jobs
                    .into_iter()
                    .filter(|job| job.tenant_id() == tenant.tenant_id)
                    .map(JobStatusDto::from_job)
                    .collect();
                Ok((StatusCode::OK, Json(ApiResponse::success(dtos))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("fetch_failed", e)),
            )),
        }
    }

    // Cancel job
    pub async fn cancel_job(
        State(handler): State<Arc<JobHandler>>,
        tenant: TenantContext,
        Path(job_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = CancelJobRequest { job_id };

        match handler
            .cancel_job_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = CancelJobResponseDto::from(response);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(CancelJobError::JobNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    ErrorCode::JobNotFound.as_str().to_string(),
                    format!("Job with ID {} not found", id),
                    None,
                )),
            )),
            Err(CancelJobError::JobNotCancellable(msg)) => Ok((
                StatusCode::CONFLICT,
                Json(ApiResponse::error(
                    ErrorCode::JobNotCancellable.as_str().to_string(),
                    msg,
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::internal_error("cancel_failed", e)),
            )),
        }
    }
}
