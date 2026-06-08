use axum::{Router, routing::get};
use std::sync::Arc;

use crate::presentation::http::handlers::SearchHandler;

pub fn search_routes(search_handler: Arc<SearchHandler>) -> Router {
    Router::new()
        .route("/search", get(SearchHandler::search_content))
        .with_state(search_handler)
}
