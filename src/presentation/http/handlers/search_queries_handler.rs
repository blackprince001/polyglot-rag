use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::domain::repositories::SearchQueryRepository;
use crate::presentation::http::dto::{
    ApiResponse, PaginationDto, SearchQueryDto, SearchQueryListResponseDto,
};
use crate::presentation::http::middleware::TenantContext;

pub struct SearchQueriesHandler {
    repository: Arc<dyn SearchQueryRepository>,
}

impl SearchQueriesHandler {
    pub fn new(repository: Arc<dyn SearchQueryRepository>) -> Self {
        Self { repository }
    }

    pub async fn list_search_queries(
        State(handler): State<Arc<SearchQueriesHandler>>,
        tenant: TenantContext,
        Query(pagination): Query<PaginationDto>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let skip = pagination.skip.max(0);
        let limit = pagination.limit.clamp(1, 100);

        match handler
            .repository
            .list_by_tenant(tenant.tenant_id, skip, limit)
            .await
        {
            Ok(queries) => {
                let dto = SearchQueryListResponseDto {
                    queries: queries.into_iter().map(SearchQueryDto::from).collect(),
                };
                Ok((StatusCode::OK, Json(ApiResponse::success(dto))))
            }
            Err(e) => Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<SearchQueryListResponseDto>::internal_error(
                    "list_search_queries_failed",
                    e,
                )),
            )),
        }
    }
}
