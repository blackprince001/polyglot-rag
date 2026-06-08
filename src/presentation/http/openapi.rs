//! OpenAPI 3.1 spec for Scalar.
//!
//! All handlers in this module are placeholder signatures consumed by
//! `#[derive(OpenApi)]` at compile time to build the path table — utoipa
//! extracts the `#[utoipa::path(...)]` metadata from each function and
//! registers it in the spec, then discards the body. The dummy bodies
//! (`{}`) and the `async fn` declarations are required by the macro; the
//! `#[allow(dead_code)]` silences the linter, which would otherwise flag
//! them as unused even though the macro consumes them at build time.

#![allow(dead_code)]

//! OpenAPI 3.1 spec for Scalar.
//!
//! All handlers in this module are placeholder signatures consumed by
//! `#[derive(OpenApi)]` at compile time to build the path table — utoipa
//! extracts the `#[utoipa::path(...)]` metadata from each function and
//! registers it in the spec, then discards the body. The dummy bodies
//! (`{}`) and the `async fn` declarations are required by the macro; the
//! `#[allow(dead_code)]` silences the linter, which would otherwise flag
//! them as unused even though the macro consumes them at build time.

#![allow(dead_code)]

use std::sync::LazyLock;

use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::presentation::http::dto::{
    ApiKeyCreatedDto, ApiKeyListResponseDto, ApiKeyScope, ApiKeySummaryDto, ApiResponse, AssetDto,
    CancelJobResponseDto, CompleteUploadResponseDto, ContentProcessingResponse,
    CreateApiKeyRequest, CreateTenantRequest, DependencyStatus, DocumentChunkDto,
    DocumentWithChunksDto, ErrorCode, FileDetailResponseDto, FileListResponseDto, FileResponseDto,
    HeaderPairDto, HealthResponseDto, JobResultDto, JobStatusDto, JobTypeDto, MessageResponseDto,
    PaginationDto, PaginationMetaDto, ProcessFileResponseDto, ProcessTextRequest,
    ProcessUrlRequest, ProcessUrlRequestDto, ProcessYoutubeRequest, ProcessYoutubeRequestDto,
    QueueJobResponseDto, RequestUploadUrlRequestDto, RequestUploadUrlResponseDto,
    SearchQueryDto, SearchQueryListResponseDto, SearchRequestDto,
    SearchResponseDto, TenantListResponseDto, TenantResponseDto, UploadResponseDto,
    UploadWithProcessingResponse,
};
use crate::presentation::http::handlers::embedding_handler::{
    SimilaritySearchRequest, SimilaritySearchResponse,
};

struct ApiKeySecurity;

impl Modify for ApiKeySecurity {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);
        components.add_security_scheme(
            "ApiKey",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-API-Key"))),
        );
        components.add_security_scheme(
            "ApiKeyBearer",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
        );
        components.add_security_scheme(
            "ManagementKey",
            SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Management-Key"))),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "PolyglotRAG API",
        version = "0.1.0",
        description = "REST API for PolyglotRAG — a multi-source personal knowledge retrieval engine. \
            All data endpoints require an API key resolved to a tenant (Authorization: Bearer <key> \
            or X-API-Key). The /scalar UI and /health endpoints are public.",
        contact(name = "blackprince001", url = "https://github.com/blackprince001"),
    ),
    servers(
        (url = "", description = "Local development server"),
    ),
    modifiers(&ApiKeySecurity),
    components(
        schemas(
            ApiResponse<HealthResponseDto>,
            ApiResponse<String>,
            ApiResponse<MessageResponseDto>,
            ApiResponse<FileResponseDto>,
            ApiResponse<FileListResponseDto>,
            ApiResponse<FileDetailResponseDto>,
            ApiResponse<UploadResponseDto>,
            ApiResponse<UploadWithProcessingResponse>,
            ApiResponse<ProcessFileResponseDto>,
            ApiResponse<ContentProcessingResponse>,
            ApiResponse<JobStatusDto>,
            ApiResponse<Vec<JobStatusDto>>,
            ApiResponse<QueueJobResponseDto>,
            ApiResponse<CancelJobResponseDto>,
            ApiResponse<SearchResponseDto>,
            ApiResponse<SearchQueryListResponseDto>,
            SearchQueryDto,
            SearchQueryListResponseDto,
            ApiResponse<DocumentChunkDto>,
            ApiResponse<DocumentWithChunksDto>,
            ApiResponse<SimilaritySearchResponse>,
            ApiResponse<TenantResponseDto>,
            ApiResponse<TenantListResponseDto>,
            ApiResponse<ApiKeyCreatedDto>,
            ApiResponse<ApiKeyListResponseDto>,
            HealthResponseDto,
            DependencyStatus,
            FileResponseDto,
            FileListResponseDto,
            FileDetailResponseDto,
            UploadResponseDto,
            UploadWithProcessingResponse,
            RequestUploadUrlRequestDto,
            RequestUploadUrlResponseDto,
            CompleteUploadResponseDto,
            HeaderPairDto,
            ProcessFileResponseDto,
            ContentProcessingResponse,
            JobStatusDto,
            JobTypeDto,
            JobResultDto,
            QueueJobResponseDto,
            CancelJobResponseDto,
            SearchResponseDto,
            SearchRequestDto,
            DocumentChunkDto,
            DocumentWithChunksDto,
            AssetDto,
            ErrorCode,
            SimilaritySearchRequest,
            SimilaritySearchResponse,
            ProcessUrlRequest,
            ProcessYoutubeRequest,
            ProcessUrlRequestDto,
            ProcessYoutubeRequestDto,
            PaginationDto,
            PaginationMetaDto,
            MessageResponseDto,
            TenantResponseDto,
            TenantListResponseDto,
            CreateTenantRequest,
            CreateApiKeyRequest,
            ApiKeyScope,
            ApiKeyCreatedDto,
            ApiKeySummaryDto,
            ApiKeyListResponseDto,
        )
    ),
    tags(
        (name = "Health", description = "Public liveness and root info endpoints"),
        (name = "Files", description = "Upload, list, fetch, update and delete files"),
        (name = "Content", description = "Process URLs and YouTube videos directly"),
        (name = "Jobs", description = "Asynchronous processing job queue and status"),
        (name = "Search", description = "Semantic and similarity search over chunks"),
        (name = "Chunks", description = "Read and delete text chunks for files"),
        (name = "Embeddings", description = "Inspect and search vector embeddings"),
        (name = "Tenants", description = "Tenant and API key provisioning. Authenticated \
            with TENANT_MANAGEMENT_KEY (not a tenant API key). These endpoints exist \
            to *create* the keys the data plane requires."),
    ),
    paths(
        health_root,
        health_check,
        upload_file,
        upload_file_with_processing,
        request_upload_url,
        complete_upload,
        get_file_content,
        get_asset_content,
        list_files,
        get_file_count,
        get_file,
        update_file,
        delete_file,
        process_file,
        process_url,
        process_text,
        process_youtube,
        queue_file_processing,
        queue_url_extraction,
        queue_youtube_extraction,
        get_job_status,
        get_file_jobs,
        get_active_jobs,
        cancel_job,
        search_content,
        list_search_queries,
        get_chunk,
        get_chunks_by_file,
        get_chunk_count_by_file,
        delete_chunk,
        delete_chunks_by_file,
        get_embedding,
        get_embedding_by_chunk,
        get_embeddings_by_file,
        similarity_search,
        delete_embedding,
        delete_embeddings_by_chunk,
        delete_embeddings_by_file,
        get_embedding_count,
        create_tenant,
        list_tenants,
        create_api_key,
        list_api_keys,
        revoke_api_key,
    )
)]
pub struct ApiDoc;

impl ApiDoc {
    pub fn openapi_json() -> &'static str {
        static CACHE: LazyLock<String> = LazyLock::new(|| {
            let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
            let port: u16 = std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000);
            let server_url = format!("http://{}:{}", host, port);

            let mut doc = <ApiDoc as utoipa::OpenApi>::openapi();
            doc.servers = Some(vec![
                utoipa::openapi::server::ServerBuilder::new()
                    .url(server_url)
                    .description(Some("Dynamic server URL from HOST/PORT env"))
                    .build(),
            ]);
            match doc.to_pretty_json() {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("openapi pretty-print failed: {e}; falling back to compact");
                    match doc.to_json() {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("openapi compact serialization failed: {e}");
                            format!(
                                r#"{{"error":"OPENAPI_SERIALIZATION_FAILED","detail":"{}"}}"#,
                                e.to_string().replace('"', "'")
                            )
                        }
                    }
                }
            }
        });
        CACHE.as_str()
    }
}

#[utoipa::path(
    get,
    path = "/",
    tag = "Health",
    summary = "API root",
    description = "Returns a static welcome payload. No authentication required.",
    responses(
        (status = 200, description = "Service is reachable", body = ApiResponse<String>),
    )
)]
async fn health_root() {}

#[utoipa::path(
    get,
    path = "/health",
    tag = "Health",
    summary = "Health check",
    description = "Returns service status, version, and a dependency map (currently \
        `embeddings`, probing the TEI service). HTTP status is always 200 if the \
        process is alive; `status` is `healthy` when every dependency is `up` and \
        `degraded` otherwise. No authentication required.",
    responses(
        (status = 200, description = "Service is alive (status field reflects dependency health)", body = ApiResponse<HealthResponseDto>),
    )
)]
async fn health_check() {}

#[utoipa::path(
    post,
    path = "/upload",
    tag = "Files",
    summary = "Upload a file",
    description = "Uploads a single file as multipart/form-data. Maximum size is 250 MB. \
        The file is stored on disk and a file_id is returned; processing is not triggered \
        automatically (use `/upload-and-process` for that, or `/single-process/{file_id}`).",
    request_body(
        content_type = "multipart/form-data",
        content = inline(serde_json::Value)
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 201, description = "File uploaded", body = ApiResponse<UploadResponseDto>),
        (status = 400, description = "No file provided or multipart parse error"),
    )
)]
async fn upload_file() {}

#[utoipa::path(
    post,
    path = "/upload-and-process",
    tag = "Files",
    summary = "Upload and process a file in one step",
    description = "Uploads a file and immediately enqueues a processing job. The response \
        includes a `job_id` and a `progress_stream_url` for SSE-based progress tracking. \
        Supports multipart fields named `file`, `upload`, or `document`, plus an optional \
        `auto_process` boolean field (default `true`).",
    request_body(
        content_type = "multipart/form-data",
        content = inline(serde_json::Value)
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 201, description = "File uploaded and job queued", body = ApiResponse<UploadWithProcessingResponse>),
        (status = 400, description = "No file provided or multipart parse error"),
    )
)]
async fn upload_file_with_processing() {}

#[utoipa::path(
    post,
    path = "/upload-url",
    tag = "Files",
    summary = "Request a presigned upload URL",
    description = "Mints a short-lived upload grant. For the local backend, \
        `url` is `null` and the client uses `POST /upload` with multipart. For \
        S3 the client `PUT`s to the returned URL with the matching `Content-Type` \
        header. For Cloudinary the client `POST`s to the returned URL with the \
        form fields (which include a signed `signature`). After uploading, the \
        client must call `POST /files/{file_id}/complete-upload` to enqueue \
        the processing job. The C9 janitor sweeps files that stay in the \
        pending state past the presigned TTL.",
    request_body = RequestUploadUrlRequestDto,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Upload grant issued", body = ApiResponse<RequestUploadUrlResponseDto>),
        (status = 400, description = "Invalid file_name"),
    )
)]
async fn request_upload_url() {}

#[utoipa::path(
    post,
    path = "/files/{file_id}/complete-upload",
    tag = "Files",
    summary = "Mark an upload complete and enqueue processing",
    description = "Called by the client after a presigned upload finishes. \
        Verifies the file row exists for the tenant, then enqueues the \
        processing job. Returns 404 if the row is missing (wrong tenant, \
        or the upload-url was never created).",
    params(
        ("file_id" = Uuid, Path, description = "File id returned by /upload-url"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Processing job enqueued", body = ApiResponse<CompleteUploadResponseDto>),
        (status = 404, description = "File not found for this tenant"),
    )
)]
async fn complete_upload() {}

#[utoipa::path(
    get,
    path = "/files/{file_id}/content",
    tag = "Files",
    summary = "Stream or redirect to the file content",
    description = "For the local backend, streams the bytes through the server \
        with the stored `Content-Type`. For S3 and Cloudinary, returns a 302 \
        redirect to a short-lived presigned GET URL (TTL configured via \
        `PRESIGNED_DOWNLOAD_TTL_SECS`). The bytes never proxy through the \
        server for object-store backends.",
    params(
        ("file_id" = Uuid, Path, description = "File id"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Bytes streamed (local backend only)", content_type = "application/octet-stream"),
        (status = 302, description = "Redirect to presigned URL (object stores)"),
        (status = 404, description = "File not found for this tenant"),
    )
)]
async fn get_file_content() {}

#[utoipa::path(
    get,
    path = "/files/{file_id}/assets/{asset_id}/content",
    tag = "Files",
    summary = "Stream or redirect to a document asset's content",
    description = "Returns the bytes of a document-derived asset (e.g. an \
        embedded image). For the local backend the bytes stream through the \
        server with the asset's stored `Content-Type`; for object stores a 302 \
        redirect to a short-lived presigned GET is returned. The asset must \
        belong to both the named file and the calling tenant.",
    params(
        ("file_id" = Uuid, Path, description = "Owning file id"),
        ("asset_id" = Uuid, Path, description = "Asset id"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Bytes streamed (local backend only)", content_type = "application/octet-stream"),
        (status = 302, description = "Redirect to presigned URL (object stores)"),
        (status = 404, description = "Asset not found for this file/tenant"),
    )
)]
async fn get_asset_content() {}

#[utoipa::path(
    get,
    path = "/files",
    tag = "Files",
    summary = "List files for the current tenant",
    params(
        ("skip" = Option<i64>, Query, description = "Number of items to skip (default 0)"),
        ("limit" = Option<i64>, Query, description = "Maximum items to return (default 20)"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Paginated file list", body = ApiResponse<FileListResponseDto>),
    )
)]
async fn list_files() {}

#[utoipa::path(
    get,
    path = "/filesys/count",
    tag = "Files",
    summary = "Count files for the current tenant",
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Total file count", body = ApiResponse<serde_json::Value>),
    )
)]
async fn get_file_count() {}

#[utoipa::path(
    get,
    path = "/files/{file_id}",
    tag = "Files",
    summary = "Get a single file by ID",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "File details", body = ApiResponse<FileDetailResponseDto>),
        (status = 404, description = "File not found"),
    )
)]
async fn get_file() {}

#[utoipa::path(
    put,
    path = "/files/{file_id}",
    tag = "Files",
    summary = "Update file metadata",
    description = "Currently a no-op placeholder that returns the current file state. \
        Accepts a JSON body describing the desired update.",
    params(("file_id" = uuid::Uuid, Path,)),
    request_body(content = serde_json::Value),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "File updated", body = ApiResponse<FileDetailResponseDto>),
        (status = 404, description = "File not found"),
    )
)]
async fn update_file() {}

#[utoipa::path(
    delete,
    path = "/files/{file_id}",
    tag = "Files",
    summary = "Delete a file (and its chunks/embeddings)",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "File deleted", body = ApiResponse<String>),
        (status = 404, description = "File not found"),
    )
)]
async fn delete_file() {}

#[utoipa::path(
    post,
    path = "/single-process/{file_id}",
    tag = "Files",
    summary = "Synchronously process a previously uploaded file",
    description = "Chunks the file, embeds the chunks, and stores vectors synchronously. \
        For long-running processing prefer the async job endpoints.",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "File processed", body = ApiResponse<ProcessFileResponseDto>),
        (status = 400, description = "Processing failed"),
    )
)]
async fn process_file() {}

#[utoipa::path(
    post,
    path = "/process/url",
    tag = "Content",
    summary = "Process a web URL into the RAG pipeline",
    description = "Fetches the URL, extracts text, and enqueues a processing job. \
        Returns a `job_id` and a `progress_stream_url` for SSE progress.",
    request_body = ProcessUrlRequest,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Job accepted", body = ApiResponse<ContentProcessingResponse>),
        (status = 400, description = "Invalid URL or validation error"),
    )
)]
async fn process_url() {}

#[utoipa::path(
    post,
    path = "/process/youtube",
    tag = "Content",
    summary = "Process a YouTube video into the RAG pipeline",
    description = "Fetches the YouTube transcript (with timestamps by default) and \
        enqueues a processing job. Returns a `job_id` and progress stream URL.",
    request_body = ProcessYoutubeRequest,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Job accepted", body = ApiResponse<ContentProcessingResponse>),
        (status = 400, description = "Invalid YouTube URL or validation error"),
    )
)]
async fn process_youtube() {}

#[utoipa::path(
    post,
    path = "/process/text",
    tag = "Content",
    summary = "Ingest a raw text blob into the RAG pipeline",
    description = "Stores the supplied UTF-8 text as a `text/plain` file, then \
        enqueues a `FileProcessing` job that chunks and embeds it. Max body \
        size is 1 MiB; for larger payloads, use `POST /upload` (multipart).",
    request_body = ProcessTextRequest,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Text accepted, job queued", body = ApiResponse<ContentProcessingResponse>),
        (status = 400, description = "Empty text or other validation error"),
    )
)]
async fn process_text() {}

#[utoipa::path(
    post,
    path = "/processing-job/file/{file_id}",
    tag = "Jobs",
    summary = "Queue a file processing job for an existing file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Job queued", body = ApiResponse<QueueJobResponseDto>),
        (status = 400, description = "Queueing failed"),
    )
)]
async fn queue_file_processing() {}

#[utoipa::path(
    post,
    path = "/processing-job/url/{file_id}",
    tag = "Jobs",
    summary = "Queue a URL extraction job against a file slot",
    params(("file_id" = uuid::Uuid, Path,)),
    request_body = ProcessUrlRequestDto,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Job queued", body = ApiResponse<QueueJobResponseDto>),
        (status = 400, description = "Queueing failed"),
    )
)]
async fn queue_url_extraction() {}

#[utoipa::path(
    post,
    path = "/processing-job/youtube/{file_id}",
    tag = "Jobs",
    summary = "Queue a YouTube extraction job against a file slot",
    params(("file_id" = uuid::Uuid, Path,)),
    request_body = ProcessYoutubeRequestDto,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 202, description = "Job queued", body = ApiResponse<QueueJobResponseDto>),
        (status = 400, description = "Queueing failed"),
    )
)]
async fn queue_youtube_extraction() {}

#[utoipa::path(
    get,
    path = "/jobs/{job_id}",
    tag = "Jobs",
    summary = "Get job status",
    params(("job_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Job status", body = ApiResponse<JobStatusDto>),
        (status = 404, description = "Job not found"),
    )
)]
async fn get_job_status() {}

#[utoipa::path(
    get,
    path = "/file-jobs/file/{file_id}",
    tag = "Jobs",
    summary = "List all jobs for a file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Jobs for the file", body = ApiResponse<Vec<JobStatusDto>>),
    )
)]
async fn get_file_jobs() {}

#[utoipa::path(
    get,
    path = "/active-jobs",
    tag = "Jobs",
    summary = "List active jobs for the current tenant",
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Active jobs", body = ApiResponse<Vec<JobStatusDto>>),
    )
)]
async fn get_active_jobs() {}

#[utoipa::path(
    delete,
    path = "/jobs/{job_id}/cancel",
    tag = "Jobs",
    summary = "Cancel a queued or running job",
    params(("job_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Job cancelled", body = ApiResponse<CancelJobResponseDto>),
        (status = 400, description = "Cancellation failed"),
    )
)]
async fn cancel_job() {}

#[utoipa::path(
    get,
    path = "/search",
    tag = "Search",
    summary = "Semantic search over chunks for the current tenant",
    description = "Embeds the query, runs vector similarity, and returns the most relevant \
        chunks grouped by source document, each chunk carrying a `similarity_score`.",
    params(
        ("query" = String, Query, description = "Natural-language search query"),
        ("limit" = Option<i32>, Query, description = "Maximum chunks to return (default 10)"),
        ("similarity_threshold" = Option<f32>, Query, description = "Minimum similarity score"),
        ("file_id" = Option<uuid::Uuid>, Query, description = "Restrict search to a single file"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Search results", body = ApiResponse<SearchResponseDto>),
        (status = 400, description = "Empty query"),
    )
)]
async fn search_content() {}

#[utoipa::path(
    get,
    path = "/search-queries",
    tag = "Search",
    summary = "List search queries for the current tenant",
    description = "Returns paginated search queries recorded for this tenant, most recent first.",
    params(
        ("skip" = Option<i64>, Query, description = "Number of records to skip (default 0)"),
        ("limit" = Option<i64>, Query, description = "Max records to return (default 20, max 100)"),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Paginated search query list", body = ApiResponse<SearchQueryListResponseDto>),
    )
)]
async fn list_search_queries() {}

#[utoipa::path(
    get,
    path = "/chunks/{chunk_id}",
    tag = "Chunks",
    summary = "Get a single chunk by ID",
    params(("chunk_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Chunk", body = ApiResponse<DocumentChunkDto>),
        (status = 404, description = "Chunk not found"),
    )
)]
async fn get_chunk() {}

#[utoipa::path(
    get,
    path = "/file-chunks/{file_id}",
    tag = "Chunks",
    summary = "List chunks for a file",
    params(
        ("file_id" = uuid::Uuid, Path,),
        ("skip" = Option<i64>, Query,),
        ("limit" = Option<i64>, Query,),
    ),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Chunks for the file", body = ApiResponse<DocumentWithChunksDto>),
        (status = 404, description = "File not found"),
    )
)]
async fn get_chunks_by_file() {}

#[utoipa::path(
    get,
    path = "/file-chunks/{file_id}/count",
    tag = "Chunks",
    summary = "Count chunks for a file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Chunk count", body = ApiResponse<serde_json::Value>),
    )
)]
async fn get_chunk_count_by_file() {}

#[utoipa::path(
    delete,
    path = "/chunks/{chunk_id}",
    tag = "Chunks",
    summary = "Delete a single chunk",
    params(("chunk_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Chunk deleted", body = ApiResponse<String>),
        (status = 404, description = "Chunk not found"),
    )
)]
async fn delete_chunk() {}

#[utoipa::path(
    delete,
    path = "/file-chunks/{file_id}",
    tag = "Chunks",
    summary = "Delete all chunks for a file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Chunks deleted", body = ApiResponse<serde_json::Value>),
    )
)]
async fn delete_chunks_by_file() {}

#[utoipa::path(
    get,
    path = "/embeddings/{embedding_id}",
    tag = "Embeddings",
    summary = "Get a single embedding by ID",
    params(("embedding_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embedding", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Embedding not found"),
    )
)]
async fn get_embedding() {}

#[utoipa::path(
    get,
    path = "/chunk-embeddings/{chunk_id}",
    tag = "Embeddings",
    summary = "Get the embedding for a chunk",
    params(("chunk_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embedding", body = ApiResponse<serde_json::Value>),
        (status = 404, description = "Embedding not found"),
    )
)]
async fn get_embedding_by_chunk() {}

#[utoipa::path(
    get,
    path = "/file-embeddings/{file_id}",
    tag = "Embeddings",
    summary = "List embeddings for a file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embeddings for the file", body = ApiResponse<serde_json::Value>),
    )
)]
async fn get_embeddings_by_file() {}

#[utoipa::path(
    post,
    path = "/similarity-search",
    tag = "Embeddings",
    summary = "Vector similarity search using a pre-computed query vector",
    description = "Run cosine/L2 similarity over the current tenant's embeddings. \
        Returns `(chunk_id, similarity_score)` pairs ordered by score.",
    request_body = SimilaritySearchRequest,
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Similarity results", body = ApiResponse<SimilaritySearchResponse>),
    )
)]
async fn similarity_search() {}

#[utoipa::path(
    delete,
    path = "/embeddings/{embedding_id}",
    tag = "Embeddings",
    summary = "Delete a single embedding",
    params(("embedding_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embedding deleted", body = ApiResponse<String>),
        (status = 404, description = "Embedding not found"),
    )
)]
async fn delete_embedding() {}

#[utoipa::path(
    delete,
    path = "/chunk-embeddings/{chunk_id}",
    tag = "Embeddings",
    summary = "Delete the embedding for a chunk",
    params(("chunk_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embeddings deleted", body = ApiResponse<String>),
        (status = 404, description = "No embedding for that chunk"),
    )
)]
async fn delete_embeddings_by_chunk() {}

#[utoipa::path(
    delete,
    path = "/file-embeddings/{file_id}",
    tag = "Embeddings",
    summary = "Delete all embeddings for a file",
    params(("file_id" = uuid::Uuid, Path,)),
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embeddings deleted", body = ApiResponse<serde_json::Value>),
    )
)]
async fn delete_embeddings_by_file() {}

#[utoipa::path(
    get,
    path = "/embeddings-count",
    tag = "Embeddings",
    summary = "Count embeddings for the current tenant",
    security(("ApiKey" = []), ("ApiKeyBearer" = [])),
    responses(
        (status = 200, description = "Embedding count", body = ApiResponse<serde_json::Value>),
    )
)]
async fn get_embedding_count() {}

#[utoipa::path(
    post,
    path = "/tenants",
    tag = "Tenants",
    summary = "Create a tenant",
    description = "Provisions a new tenant. Authenticated with TENANT_MANAGEMENT_KEY, \
        not a tenant API key (since this endpoint creates the tenants that the API \
        keys are scoped to).",
    request_body = CreateTenantRequest,
    security(("ManagementKey" = [])),
    responses(
        (status = 201, description = "Tenant created", body = ApiResponse<TenantResponseDto>),
        (status = 400, description = "Validation error (e.g. empty name)"),
        (status = 401, description = "Missing or invalid management key"),
        (status = 503, description = "TENANT_MANAGEMENT_KEY is not configured"),
    )
)]
async fn create_tenant() {}

#[utoipa::path(
    get,
    path = "/tenants",
    tag = "Tenants",
    summary = "List tenants",
    security(("ManagementKey" = [])),
    responses(
        (status = 200, description = "Tenant list", body = ApiResponse<TenantListResponseDto>),
        (status = 401, description = "Missing or invalid management key"),
        (status = 503, description = "TENANT_MANAGEMENT_KEY is not configured"),
    )
)]
async fn list_tenants() {}

#[utoipa::path(
    post,
    path = "/tenants/{tenant_id}/keys",
    tag = "Tenants",
    summary = "Create an API key for a tenant",
    description = "Generates a new API key, persists its SHA-256 hash, and returns \
        the raw secret **once**. Store it immediately — subsequent reads of the \
        key list only expose the prefix and metadata.",
    params(("tenant_id" = uuid::Uuid, Path,)),
    request_body = CreateApiKeyRequest,
    security(("ManagementKey" = [])),
    responses(
        (status = 201, description = "API key created (raw key returned once)", body = ApiResponse<ApiKeyCreatedDto>),
        (status = 404, description = "Tenant not found or inactive"),
        (status = 401, description = "Missing or invalid management key"),
        (status = 503, description = "TENANT_MANAGEMENT_KEY is not configured"),
    )
)]
async fn create_api_key() {}

#[utoipa::path(
    get,
    path = "/tenants/{tenant_id}/keys",
    tag = "Tenants",
    summary = "List API keys for a tenant (metadata only)",
    description = "Returns the prefix, scopes, and audit timestamps for every key \
        belonging to the tenant — both active and revoked. The raw secret and its \
        hash are never returned here.",
    params(("tenant_id" = uuid::Uuid, Path,)),
    security(("ManagementKey" = [])),
    responses(
        (status = 200, description = "API key metadata", body = ApiResponse<ApiKeyListResponseDto>),
        (status = 401, description = "Missing or invalid management key"),
        (status = 503, description = "TENANT_MANAGEMENT_KEY is not configured"),
    )
)]
async fn list_api_keys() {}

#[utoipa::path(
    delete,
    path = "/tenants/{tenant_id}/keys/{api_key_id}",
    tag = "Tenants",
    summary = "Revoke an API key",
    description = "Sets `revoked_at` on the key. The key stops being accepted on \
        subsequent requests immediately. Idempotent: revoking an already-revoked \
        or non-existent key returns 404.",
    params(
        ("tenant_id" = uuid::Uuid, Path,),
        ("api_key_id" = uuid::Uuid, Path,),
    ),
    security(("ManagementKey" = [])),
    responses(
        (status = 200, description = "API key revoked", body = ApiResponse<String>),
        (status = 404, description = "No active key with that id for this tenant"),
        (status = 401, description = "Missing or invalid management key"),
        (status = 503, description = "TENANT_MANAGEMENT_KEY is not configured"),
    )
)]
async fn revoke_api_key() {}
