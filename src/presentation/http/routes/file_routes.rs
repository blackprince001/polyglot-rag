use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::presentation::http::handlers::FileHandler;

pub fn file_routes(file_handler: Arc<FileHandler>) -> Router {
    Router::new()
        .route("/upload", post(FileHandler::upload_file))
        .route(
            "/upload-and-process",
            post(FileHandler::upload_file_with_processing),
        )
        .route("/upload-url", post(FileHandler::request_upload_url))
        .route("/files", get(FileHandler::list_files))
        .route("/filesys/count", get(FileHandler::get_file_count))
        .route("/files/{file_id}", get(FileHandler::get_file))
        .route(
            "/files/{file_id}/content",
            get(FileHandler::get_file_content),
        )
        .route(
            "/files/{file_id}/assets/{asset_id}/content",
            get(FileHandler::get_asset_content),
        )
        .route(
            "/files/{file_id}/complete-upload",
            post(FileHandler::complete_upload),
        )
        .route("/files/{file_id}", put(FileHandler::update_file))
        .route("/files/{file_id}", delete(FileHandler::delete_file))
        .route("/single-process/{file_id}", post(FileHandler::process_file))
        .with_state(file_handler)
}
