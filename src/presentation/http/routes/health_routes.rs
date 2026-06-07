use std::sync::Arc;

use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::get};
use scalar_api_reference::axum::router as scalar_router;
use serde_json::json;

use crate::presentation::http::dto::ApiResponse;
use crate::presentation::http::handlers::HealthHandler;
use crate::presentation::http::openapi::ApiDoc;

pub fn health_routes(health_handler: Arc<HealthHandler>) -> Router {
    let scalar_config = json!({
        "url": "/openapi.json",
        "title": "PolyglotRAG API",
        "theme": "purple",
        "hideModels": false,
    });

    let stateful = Router::new()
        .route("/health", get(HealthHandler::health))
        .with_state(health_handler);

    Router::new()
        .route("/", get(root_handler))
        .merge(stateful)
        .merge(scalar_router("/scalar", &scalar_config))
        .route("/openapi.json", get(openapi_json_handler))
}

async fn root_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(ApiResponse::success("PolyglotRAG".to_string())),
    )
}

async fn openapi_json_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "application/json")],
        ApiDoc::openapi_json(),
    )
}
