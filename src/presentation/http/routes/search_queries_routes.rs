use axum::{Router, routing::get};
use std::sync::Arc;

use crate::presentation::http::handlers::SearchQueriesHandler;

pub fn search_queries_routes(handler: Arc<SearchQueriesHandler>) -> Router {
    Router::new()
        .route("/search-queries", get(SearchQueriesHandler::list_search_queries))
        .with_state(handler)
}
