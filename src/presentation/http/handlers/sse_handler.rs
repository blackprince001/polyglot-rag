use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
};
use futures::stream::{self, Stream};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::time::sleep;
use uuid::Uuid;

use crate::application::use_cases::{GetJobStatusUseCase, get_job_status::GetJobStatusRequest};
use crate::presentation::http::dto::job_dto::JobStatusDto;
use crate::presentation::http::middleware::TenantContext;

pub struct SseHandler {
    get_job_status_use_case: Arc<GetJobStatusUseCase>,
}

impl SseHandler {
    pub fn new(get_job_status_use_case: Arc<GetJobStatusUseCase>) -> Self {
        Self {
            get_job_status_use_case,
        }
    }

    pub async fn job_progress_stream(
        State(handler): State<Arc<SseHandler>>,
        tenant: TenantContext,
        Path(job_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let use_case = handler.get_job_status_use_case.clone();
        let tenant_id = tenant.tenant_id;

        let stream = stream::unfold(Some(()), move |state| {
            let use_case = use_case.clone();
            async move {
                if state.is_none() {
                    return None; // Stream ended
                }

                // Get current job status
                let request = GetJobStatusRequest { job_id };

                match use_case.execute(tenant_id, request).await {
                    Ok(response) => {
                        let job_status = JobStatusDto::from(response);
                        let event_data = serde_json::to_string(&job_status).unwrap_or_default();

                        // Create SSE event
                        let event = axum::response::sse::Event::default()
                            .event("job_progress")
                            .data(event_data);

                        // If job is complete, send final event and stop
                        if job_status.is_terminal {
                            Some((Ok::<_, std::convert::Infallible>(event), None)) // None stops the stream
                        } else {
                            // Continue streaming with delay
                            sleep(Duration::from_secs(1)).await;
                            Some((Ok::<_, std::convert::Infallible>(event), Some(())))
                        }
                    }
                    Err(_) => {
                        // Job not found or error - send error event and stop
                        let error_event = axum::response::sse::Event::default()
                            .event("error")
                            .data(format!("Job {} not found", job_id));

                        Some((Ok::<_, std::convert::Infallible>(error_event), None))
                    }
                }
            }
        });

        Ok(create_sse_response(stream))
    }

    pub async fn multiple_jobs_stream(
        State(handler): State<Arc<SseHandler>>,
        tenant: TenantContext,
    ) -> Result<impl IntoResponse, StatusCode> {
        let use_case = handler.get_job_status_use_case.clone();
        let tenant_id = tenant.tenant_id;

        let stream = stream::unfold(Some(()), move |state| {
            let use_case = use_case.clone();
            async move {
                if state.is_none() {
                    return None; // Stream ended
                }

                // Get all active jobs (scoped to this tenant)
                match use_case.get_active_jobs().await {
                    Ok(jobs) => {
                        let jobs_data: Vec<JobStatusDto> = jobs
                            .into_iter()
                            .filter(|job| job.tenant_id() == tenant_id)
                            .map(|job| JobStatusDto::from_job(job))
                            .collect();

                        let event_data = serde_json::to_string(&jobs_data).unwrap_or_default();

                        let event = axum::response::sse::Event::default()
                            .event("active_jobs")
                            .data(event_data);

                        sleep(Duration::from_secs(2)).await;
                        Some((Ok::<_, std::convert::Infallible>(event), Some(())))
                    }
                    Err(_) => {
                        let error_event = axum::response::sse::Event::default()
                            .event("error")
                            .data("Failed to fetch active jobs");

                        sleep(Duration::from_secs(5)).await;
                        Some((Ok::<_, std::convert::Infallible>(error_event), Some(())))
                    }
                }
            }
        });

        Ok(create_sse_response(stream))
    }
}

// Helper function to create SSE response with CORS headers
pub fn create_sse_response<S>(stream: S) -> Response
where
    S: Stream<Item = Result<axum::response::sse::Event, Infallible>> + Send + 'static,
{
    Sse::new(stream)
        .keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(30))
                .text("keep-alive"),
        )
        .into_response()
}
