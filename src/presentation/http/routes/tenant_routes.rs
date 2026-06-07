use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::presentation::http::handlers::TenantsHandler;

pub fn tenant_routes(tenants_handler: Arc<TenantsHandler>) -> Router {
    Router::new()
        .route("/tenants", post(TenantsHandler::create_tenant))
        .route("/tenants", get(TenantsHandler::list_tenants))
        .route(
            "/tenants/{tenant_id}/keys",
            post(TenantsHandler::create_api_key),
        )
        .route(
            "/tenants/{tenant_id}/keys",
            get(TenantsHandler::list_api_keys),
        )
        .route(
            "/tenants/{tenant_id}/keys/{api_key_id}",
            delete(TenantsHandler::revoke_api_key),
        )
        .with_state(tenants_handler)
}
