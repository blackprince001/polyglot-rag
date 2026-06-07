use axum::{Router, routing::post};
use std::sync::Arc;

use crate::presentation::http::handlers::ContentHandler;

pub fn content_processing_routes(content_handler: Arc<ContentHandler>) -> Router {
    Router::new()
        .route("/process/text", post(ContentHandler::process_text))
        .route("/process/url", post(ContentHandler::process_url))
        .route("/process/youtube", post(ContentHandler::process_youtube))
        .with_state(content_handler)
}
