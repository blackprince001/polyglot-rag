use axum::{
    Json,
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::FileStorage;
use crate::application::use_cases::{
    CompleteUploadUseCase, GetFileUseCase, ListFilesUseCase, ProcessDocumentUseCase,
    RequestUploadUrlUseCase, UploadFileUseCase, UploadWithProcessingUseCase, complete_upload,
    complete_upload::CompleteUploadRequest, get_file::GetFileError, get_file::GetFileRequest,
    list_files::ListFilesError, list_files::ListFilesRequest,
    process_document::ProcessDocumentError, process_document::ProcessDocumentRequest,
    request_upload_url, request_upload_url::RequestUploadUrlRequest, upload_file::UploadFileError,
    upload_file::UploadFileRequest, upload_with_processing::UploadWithProcessingError,
    upload_with_processing::UploadWithProcessingRequest,
};
use crate::domain::repositories::{AssetRepository, FileRepository};
use crate::presentation::http::dto::content_dto::UploadWithProcessingResponse;
use crate::presentation::http::dto::{
    ApiResponse, PaginationDto, PaginationMetaDto,
    file_dto::{
        CompleteUploadResponseDto, FileDetailResponseDto, FileListResponseDto, FileResponseDto,
        HeaderPairDto, ProcessFileResponseDto, RequestUploadUrlRequestDto,
        RequestUploadUrlResponseDto, UploadResponseDto,
    },
};
use crate::presentation::http::middleware::TenantContext;

pub struct FileHandler {
    upload_use_case: Arc<UploadFileUseCase>,
    upload_with_processing_use_case: Arc<UploadWithProcessingUseCase>,
    list_files_use_case: Arc<ListFilesUseCase>,
    process_document_use_case: Arc<ProcessDocumentUseCase>,
    get_file_use_case: Arc<GetFileUseCase>,
    request_upload_url_use_case: Arc<RequestUploadUrlUseCase>,
    complete_upload_use_case: Arc<CompleteUploadUseCase>,
    file_repository: Arc<dyn FileRepository>,
    asset_repository: Arc<dyn AssetRepository>,
    file_storage: Arc<dyn FileStorage>,
    presigned_download_ttl_secs: u64,
}

impl FileHandler {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        upload_use_case: Arc<UploadFileUseCase>,
        upload_with_processing_use_case: Arc<UploadWithProcessingUseCase>,
        list_files_use_case: Arc<ListFilesUseCase>,
        process_document_use_case: Arc<ProcessDocumentUseCase>,
        get_file_use_case: Arc<GetFileUseCase>,
        request_upload_url_use_case: Arc<RequestUploadUrlUseCase>,
        complete_upload_use_case: Arc<CompleteUploadUseCase>,
        file_repository: Arc<dyn FileRepository>,
        asset_repository: Arc<dyn AssetRepository>,
        file_storage: Arc<dyn FileStorage>,
        presigned_download_ttl_secs: u64,
    ) -> Self {
        Self {
            upload_use_case,
            upload_with_processing_use_case,
            list_files_use_case,
            process_document_use_case,
            get_file_use_case,
            request_upload_url_use_case,
            complete_upload_use_case,
            file_repository,
            asset_repository,
            file_storage,
            presigned_download_ttl_secs,
        }
    }

    pub async fn upload_file(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        mut multipart: Multipart,
    ) -> Result<impl IntoResponse, StatusCode> {
        while let Some(field) = multipart.next_field().await.map_err(|e| {
            eprintln!("Error reading multipart field: {:?}", e);
            StatusCode::BAD_REQUEST
        })? {
            let file_name = field
                .file_name()
                .ok_or_else(|| {
                    eprintln!("No file name provided in field: {:?}", field.name());
                    StatusCode::BAD_REQUEST
                })?
                .to_string();

            let content_type = field.content_type().map(|ct| ct.to_string());

            let data = field
                .bytes()
                .await
                .map_err(|e| {
                    eprintln!("Error reading file data: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?
                .to_vec();

            let request = UploadFileRequest {
                file_name,
                file_data: data,
                content_type,
                metadata: None,
            };

            match handler
                .upload_use_case
                .execute(tenant.tenant_id, request)
                .await
            {
                Ok(response) => {
                    let dto = UploadResponseDto::from(response);
                    return Ok((StatusCode::CREATED, Json(ApiResponse::success(dto))));
                }
                Err(UploadFileError::ValidationError(msg))
                | Err(UploadFileError::DuplicateFile(msg)) => {
                    return Ok((
                        StatusCode::BAD_REQUEST,
                        Json(ApiResponse::error(
                            "UPLOAD_VALIDATION_FAILED".to_string(),
                            msg,
                            None,
                        )),
                    ));
                }
                Err(e) => {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::error(
                            "UPLOAD_FAILED".to_string(),
                            e.to_string(),
                            None,
                        )),
                    ));
                }
            }
        }

        Ok((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(
                "NO_FILE_PROVIDED".to_string(),
                "No file provided in the request".to_string(),
                None,
            )),
        ))
    }

    pub async fn list_files(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Query(pagination): Query<PaginationDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = ListFilesRequest {
            skip: pagination.skip,
            limit: pagination.limit,
        };

        match handler
            .list_files_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let files: Vec<FileResponseDto> = response
                    .files
                    .into_iter()
                    .map(FileResponseDto::from)
                    .collect();

                let dto = FileListResponseDto {
                    files,
                    meta: PaginationMetaDto {
                        offset: response.skip,
                        limit: response.limit,
                        total: response.total_count,
                    },
                };

                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(ListFilesError::ValidationError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<FileListResponseDto>::error(
                    "LIST_VALIDATION_FAILED".to_string(),
                    msg,
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<FileListResponseDto>::error(
                    "LIST_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn process_file(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = ProcessDocumentRequest {
            file_id,
            extraction_options: None,
        };

        match handler
            .process_document_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = ProcessFileResponseDto::from(response);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(ProcessDocumentError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<ProcessFileResponseDto>::error(
                    "FILE_NOT_FOUND".to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(ProcessDocumentError::FileNotProcessable(msg)) => Ok((
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(ApiResponse::<ProcessFileResponseDto>::error(
                    "FILE_NOT_PROCESSABLE".to_string(),
                    msg,
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<ProcessFileResponseDto>::error(
                    "PROCESSING_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn get_file(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = GetFileRequest { file_id };

        match handler
            .get_file_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = FileDetailResponseDto::from(response);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(GetFileError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "FILE_NOT_FOUND".to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "FILE_LOOKUP_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn request_upload_url(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Json(request): Json<RequestUploadUrlRequestDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let use_case_request = RequestUploadUrlRequest {
            file_name: request.file_name,
            content_type: request.content_type,
            expiry_secs: request.expiry_secs,
        };
        match handler
            .request_upload_url_use_case
            .execute(tenant.tenant_id, use_case_request)
            .await
        {
            Ok(response) => {
                let dto = RequestUploadUrlResponseDto {
                    file_id: response.file_id,
                    file_name: response.file_name,
                    method: response.method,
                    url: response.url,
                    headers: response
                        .headers
                        .into_iter()
                        .map(HeaderPairDto::from)
                        .collect(),
                    form_fields: response
                        .form_fields
                        .into_iter()
                        .map(HeaderPairDto::from)
                        .collect(),
                    expires_at: response.expires_at.to_rfc3339(),
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(request_upload_url::RequestUploadUrlError::ValidationError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error("INVALID_REQUEST".to_string(), msg, None)),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "UPLOAD_URL_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn complete_upload(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let request = CompleteUploadRequest { file_id };
        match handler
            .complete_upload_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = CompleteUploadResponseDto {
                    file_id: response.file_id,
                    job_id: response.job_id,
                    status: response.status,
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(complete_upload::CompleteUploadError::FileNotFound(id)) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "FILE_NOT_FOUND".to_string(),
                    format!("File with ID {} not found", id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "COMPLETE_UPLOAD_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn get_file_content(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<Response, StatusCode> {
        // Tenant-scoped lookup. Wrong-tenant / not-found are both 404.
        let file = match handler
            .file_repository
            .find_by_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(Some(f)) => f,
            Ok(None) => {
                return Ok((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(
                        "FILE_NOT_FOUND".to_string(),
                        format!("File with ID {} not found", file_id),
                        None,
                    )),
                )
                    .into_response());
            }
            Err(e) => {
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(
                        "FILE_LOOKUP_FAILED".to_string(),
                        e.to_string(),
                        None,
                    )),
                )
                    .into_response());
            }
        };

        if handler.file_storage.supports_server_stream() {
            // Local backend: stream bytes through the server.
            let stream = match handler
                .file_storage
                .open_read(tenant.tenant_id, file_id)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "STREAM_FAILED".to_string(),
                            e.to_string(),
                            None,
                        )),
                    )
                        .into_response());
                }
            };
            let body = Body::from_stream(tokio_util::io::ReaderStream::new(stream));
            let mut headers = HeaderMap::new();
            if let Some(ct) = file.file_type() {
                if let Ok(v) = HeaderValue::from_str(ct) {
                    headers.insert(header::CONTENT_TYPE, v);
                }
            }
            Ok((StatusCode::OK, headers, body).into_response())
        } else {
            // Object stores: 302 redirect to a short-lived presigned GET.
            let expiry = std::time::Duration::from_secs(handler.presigned_download_ttl_secs);
            let presigned = match handler
                .file_storage
                .presigned_download_url(tenant.tenant_id, file_id, expiry)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "PRESIGN_FAILED".to_string(),
                            e.to_string(),
                            None,
                        )),
                    )
                        .into_response());
                }
            };
            let mut headers = HeaderMap::new();
            if let Ok(v) = HeaderValue::from_str(&presigned.url) {
                headers.insert(header::LOCATION, v);
            }
            Ok((StatusCode::FOUND, headers).into_response())
        }
    }

    /// Stream (or redirect to) the bytes of a document-derived asset. Mirrors
    /// `get_file_content`: local backend streams through the server, object
    /// stores 302-redirect to a presigned GET. The asset must belong to both
    /// the named file and the calling tenant.
    pub async fn get_asset_content(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path((file_id, asset_id)): Path<(Uuid, Uuid)>,
    ) -> Result<Response, StatusCode> {
        let asset = match handler
            .asset_repository
            .find_by_id(tenant.tenant_id, asset_id)
            .await
        {
            Ok(Some(a)) if a.file_id() == file_id => a,
            Ok(_) => {
                return Ok((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error(
                        "ASSET_NOT_FOUND".to_string(),
                        format!("Asset {} not found for file {}", asset_id, file_id),
                        None,
                    )),
                )
                    .into_response());
            }
            Err(e) => {
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(
                        "ASSET_LOOKUP_FAILED".to_string(),
                        e.to_string(),
                        None,
                    )),
                )
                    .into_response());
            }
        };

        // Assets are stored keyed by their own id (see DocumentProcessor).
        if handler.file_storage.supports_server_stream() {
            let stream = match handler
                .file_storage
                .open_read(tenant.tenant_id, asset_id)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "STREAM_FAILED".to_string(),
                            e.to_string(),
                            None,
                        )),
                    )
                        .into_response());
                }
            };
            let body = Body::from_stream(tokio_util::io::ReaderStream::new(stream));
            let mut headers = HeaderMap::new();
            if let Ok(v) = HeaderValue::from_str(asset.content_type()) {
                headers.insert(header::CONTENT_TYPE, v);
            }
            Ok((StatusCode::OK, headers, body).into_response())
        } else {
            let expiry = std::time::Duration::from_secs(handler.presigned_download_ttl_secs);
            let presigned = match handler
                .file_storage
                .presigned_download_url(tenant.tenant_id, asset_id, expiry)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "PRESIGN_FAILED".to_string(),
                            e.to_string(),
                            None,
                        )),
                    )
                        .into_response());
                }
            };
            let mut headers = HeaderMap::new();
            if let Ok(v) = HeaderValue::from_str(&presigned.url) {
                headers.insert(header::LOCATION, v);
            }
            Ok((StatusCode::FOUND, headers).into_response())
        }
    }

    pub async fn update_file(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
        Json(_update_request): Json<serde_json::Value>,
    ) -> Result<impl IntoResponse, StatusCode> {
        // Get the current file
        let file = match handler
            .file_repository
            .find_by_id(tenant.tenant_id, file_id)
            .await
        {
            Ok(Some(f)) => f,
            Ok(None) => {
                return Ok((
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::error(
                        "FILE_NOT_FOUND".to_string(),
                        format!("File with ID {} not found", file_id),
                        None,
                    )),
                ));
            }
            Err(e) => {
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::error(
                        "DATABASE_ERROR".to_string(),
                        e.to_string(),
                        None,
                    )),
                ));
            }
        };

        match handler
            .file_repository
            .update(tenant.tenant_id, &file)
            .await
        {
            Ok(_) => {
                let response =
                    crate::application::use_cases::get_file::GetFileResponse { file: file.clone() };
                let dto = FileDetailResponseDto::from(response);
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "UPDATE_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn delete_file(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        Path(file_id): Path<Uuid>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let assets = handler
            .asset_repository
            .find_by_file_id(tenant.tenant_id, file_id)
            .await
            .unwrap_or_default();

        match handler
            .file_repository
            .delete(tenant.tenant_id, file_id)
            .await
        {
            Ok(true) => {
                if let Err(e) = handler
                    .asset_repository
                    .delete_by_file_id(tenant.tenant_id, file_id)
                    .await
                {
                    eprintln!(
                        "[delete_file] asset row delete failed for {} (tenant={}): {}",
                        file_id, tenant.tenant_id, e
                    );
                }
                for asset in &assets {
                    if let Err(e) = handler
                        .file_storage
                        .delete(tenant.tenant_id, asset.id())
                        .await
                    {
                        eprintln!(
                            "[delete_file] asset storage delete failed for {} (tenant={}): {}",
                            asset.id(),
                            tenant.tenant_id,
                            e
                        );
                    }
                }
                if let Err(e) = handler.file_storage.delete(tenant.tenant_id, file_id).await {
                    eprintln!(
                        "[delete_file] storage delete failed for {} (tenant={}): {}",
                        file_id, tenant.tenant_id, e
                    );
                }
                Ok((
                    StatusCode::OK,
                    Json(ApiResponse::success(
                        "File deleted successfully".to_string(),
                    )),
                ))
            }
            Ok(false) => Ok((
                StatusCode::NOT_FOUND,
                Json(ApiResponse::error(
                    "FILE_NOT_FOUND".to_string(),
                    format!("File with ID {} not found", file_id),
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "DELETE_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn get_file_count(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
    ) -> Result<impl IntoResponse, StatusCode> {
        match handler.file_repository.count(tenant.tenant_id).await {
            Ok(count) => Ok((
                StatusCode::OK,
                Json(ApiResponse::success(serde_json::json!({
                    "count": count
                }))),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "COUNT_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }

    pub async fn upload_file_with_processing(
        State(handler): State<Arc<FileHandler>>,
        tenant: TenantContext,
        mut multipart: Multipart,
    ) -> Result<impl IntoResponse, StatusCode> {
        // Parse auto_process parameter (default: true)
        let mut auto_process = true;
        let mut file_data = None;
        let mut file_name = None;
        let mut content_type = None;

        while let Some(field) = multipart.next_field().await.map_err(|e| {
            eprintln!("Error reading multipart field: {:?}", e);
            StatusCode::BAD_REQUEST
        })? {
            match field.name() {
                Some("file") | Some("upload") | Some("document") => {
                    file_name = Some(
                        field
                            .file_name()
                            .ok_or_else(|| {
                                eprintln!("No file name provided in field: {:?}", field.name());
                                StatusCode::BAD_REQUEST
                            })?
                            .to_string(),
                    );

                    content_type = field.content_type().map(|ct| ct.to_string());

                    file_data = Some(
                        field
                            .bytes()
                            .await
                            .map_err(|e| {
                                eprintln!("Error reading file data: {:?}", e);
                                StatusCode::BAD_REQUEST
                            })?
                            .to_vec(),
                    );
                }
                Some("auto_process") => {
                    if let Ok(data) = field.bytes().await {
                        if let Ok(value) = String::from_utf8(data.to_vec()) {
                            auto_process = value.parse().unwrap_or(true);
                        }
                    }
                }
                _ => {
                    // Skip unknown fields
                    eprintln!("Skipping unknown field: {:?}", field.name());
                }
            }
        }

        let file_data = file_data.ok_or_else(|| {
            eprintln!("No file data found in multipart form. Expected field named 'file', 'upload', or 'document'");
            StatusCode::BAD_REQUEST
        })?;
        let file_name = file_name.ok_or_else(|| {
            eprintln!("No file name found in multipart form");
            StatusCode::BAD_REQUEST
        })?;

        let request = UploadWithProcessingRequest {
            file_data,
            file_name,
            content_type,
            auto_process,
            metadata: None,
        };

        match handler
            .upload_with_processing_use_case
            .execute(tenant.tenant_id, request)
            .await
        {
            Ok(response) => {
                let dto = UploadWithProcessingResponse::from(response);
                Ok((StatusCode::CREATED, Json(ApiResponse::success(dto))))
            }
            Err(UploadWithProcessingError::UploadError(msg)) => Ok((
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::error(
                    "UPLOAD_WITH_PROCESSING_VALIDATION_FAILED".to_string(),
                    msg,
                    None,
                )),
            )),
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error(
                    "UPLOAD_WITH_PROCESSING_FAILED".to_string(),
                    e.to_string(),
                    None,
                )),
            )),
        }
    }
}
