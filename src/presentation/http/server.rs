use axum::Router;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::domain::repositories::AuthRepository;
use crate::infrastructure::messaging::BackgroundProcessor;
use crate::presentation::http::middleware::{require_api_key, require_management_key};
use crate::presentation::http::{
    handlers::{
        ChunkHandler, ContentHandler, EmbeddingHandler, FileHandler, HealthHandler, JobHandler,
        SearchHandler, SearchQueriesHandler, SseHandler, TenantsHandler,
    },
    routes::{
        chunk_routes, content_processing_routes, embedding_routes, file_routes, health_routes,
        job_routes, search_queries_routes, search_routes, tenant_routes,
    },
};

pub struct HttpServer {
    file_handler: Arc<FileHandler>,
    content_handler: Arc<ContentHandler>,
    search_handler: Arc<SearchHandler>,
    search_queries_handler: Arc<SearchQueriesHandler>,
    job_handler: Arc<JobHandler>,
    sse_handler: Arc<SseHandler>,
    chunk_handler: Arc<ChunkHandler>,
    embedding_handler: Arc<EmbeddingHandler>,
    health_handler: Arc<HealthHandler>,
    tenants_handler: Arc<TenantsHandler>,
    background_processor: Arc<BackgroundProcessor>,
    auth_repository: Arc<dyn AuthRepository>,
    host: String,
    port: u16,
}

impl HttpServer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        file_handler: Arc<FileHandler>,
        content_handler: Arc<ContentHandler>,
        search_handler: Arc<SearchHandler>,
        search_queries_handler: Arc<SearchQueriesHandler>,
        job_handler: Arc<JobHandler>,
        sse_handler: Arc<SseHandler>,
        chunk_handler: Arc<ChunkHandler>,
        embedding_handler: Arc<EmbeddingHandler>,
        health_handler: Arc<HealthHandler>,
        tenants_handler: Arc<TenantsHandler>,
        background_processor: Arc<BackgroundProcessor>,
        auth_repository: Arc<dyn AuthRepository>,
        host: Option<String>,
        port: Option<u16>,
    ) -> Self {
        Self {
            file_handler,
            content_handler,
            search_handler,
            search_queries_handler,
            job_handler,
            sse_handler,
            chunk_handler,
            embedding_handler,
            health_handler,
            tenants_handler,
            background_processor,
            auth_repository,
            host: host.unwrap_or_else(|| "0.0.0.0".to_string()),
            port: port.unwrap_or(3000),
        }
    }

    fn cors_layer() -> CorsLayer {
        match env::var("CORS_ALLOWED_ORIGINS") {
            Ok(origins) if !origins.trim().is_empty() => {
                let parsed: Vec<_> = origins
                    .split(',')
                    .filter_map(|o| o.trim().parse().ok())
                    .collect();
                tracing::info!("CORS restricted to {} configured origin(s)", parsed.len());
                CorsLayer::new()
                    .allow_origin(parsed)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
            _ => {
                tracing::warn!(
                    "CORS_ALLOWED_ORIGINS not set; allowing any origin (development default)"
                );
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
            }
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let background_processor = self.background_processor.clone();
        tokio::spawn(async move {
            background_processor.start().await;
        });

        let management_cfg = crate::presentation::http::middleware::ManagementKeyConfig::from_env();
        let management = tenant_routes(self.tenants_handler.clone()).layer(
            axum::middleware::from_fn_with_state(management_cfg.clone(), require_management_key),
        );

        let protected = Router::new()
            .merge(file_routes(self.file_handler.clone()))
            .merge(content_processing_routes(self.content_handler))
            .merge(search_routes(self.search_handler))
            .merge(search_queries_routes(self.search_queries_handler))
            .merge(job_routes(self.job_handler, self.sse_handler))
            .merge(chunk_routes(self.chunk_handler.clone()))
            .merge(embedding_routes(self.embedding_handler.clone()))
            .layer(axum::middleware::from_fn_with_state(
                self.auth_repository.clone(),
                require_api_key,
            ));

        let app = Router::new()
            .merge(health_routes(self.health_handler.clone())) // public
            .merge(management) // TENANT_MANAGEMENT_KEY
            .merge(protected)
            .layer(Self::cors_layer())
            .layer(RequestBodyLimitLayer::new(250 * 1024 * 1024)) // 250MB cap
            .layer(
                TraceLayer::new_for_http()
                    .on_request(
                        |request: &axum::http::Request<axum::body::Body>, _span: &tracing::Span| {
                            tracing::info!(
                                "Received request: {} {}",
                                request.method(),
                                request.uri()
                            );
                        },
                    )
                    .on_response(
                        |response: &axum::http::Response<axum::body::Body>,
                         latency: std::time::Duration,
                         _span: &tracing::Span| {
                            tracing::info!(
                                "Response: {} (took {} ms)",
                                response.status(),
                                latency.as_millis()
                            );
                        },
                    )
                    .on_failure(
                        |error: ServerErrorsFailureClass,
                         latency: std::time::Duration,
                         _span: &tracing::Span| {
                            tracing::error!(
                                "Request failed: {:?} (took {} ms)",
                                error,
                                latency.as_millis()
                            );
                        },
                    ),
            );

        let host_for_log = self.host.clone();
        let port_for_log = self.port;
        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| format!("Invalid HOST/PORT ({}:{}): {}", self.host, self.port, e))?;

        let listener = TcpListener::bind(addr).await?;
        let bound_addr = listener.local_addr()?;

        // Friendly startup banner — visible regardless of tracing subscriber state.
        print_startup_banner(bound_addr, &host_for_log, port_for_log);

        axum::serve(listener, app).await?;

        Ok(())
    }
}

fn print_startup_banner(bound_addr: SocketAddr, host_cfg: &str, port_cfg: u16) {
    let display_host = match bound_addr.ip().is_unspecified() {
        true => "0.0.0.0".to_string(),
        false => bound_addr.ip().to_string(),
    };
    let display_port = bound_addr.port();

    let db_target = env::var("DATABASE_URL")
        .ok()
        .and_then(|url| redact_db_url(&url))
        .unwrap_or_else(|| "(unset)".to_string());

    let embed_target = env::var("EMBEDDINGS_SERVICE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "(unset — embedding routes will fail)".to_string());

    let upload_dir = env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string());

    println!("bind       :  {}:{}", display_host, display_port);
    if display_port != port_cfg || display_host != host_cfg {
        println!(
            "requested  :  {}:{}  (effective: {}:{})",
            host_cfg, port_cfg, display_host, display_port
        );
    }
    println!("database   :  {}", db_target);
    println!("embeddings :  {}", embed_target);
    println!("upload dir :  {}", upload_dir);
}

fn redact_db_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let after_scheme = trimmed
        .strip_prefix("postgres://")
        .or_else(|| trimmed.strip_prefix("postgresql://"))
        .unwrap_or(trimmed);

    let (authority, path) = match after_scheme.find(|c: char| c == '/' || c == '?') {
        Some(idx) => (&after_scheme[..idx], &after_scheme[idx..]),
        None => (after_scheme, ""),
    };

    let host_port = match authority.rfind('@') {
        Some(at) => &authority[at + 1..],
        None => authority,
    };

    let path = path.split('?').next().unwrap_or("");
    Some(format!("{}{}", host_port, path))
}
