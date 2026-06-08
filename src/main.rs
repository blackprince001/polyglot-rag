use std::env;
mod application;
mod domain;
mod infrastructure;
mod presentation;

use infrastructure::container::AppContainer;
use presentation::http::server::HttpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    dotenv::dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let worker_count: usize = match env::var("WORKER_COUNT") {
        Ok(raw) => match raw.trim().parse::<usize>() {
            Ok(n) if n >= 1 => n,
            Ok(_) => {
                eprintln!(
                    "WORKER_COUNT={} is invalid; must be >= 1. Falling back to 3.",
                    raw
                );
                3
            }
            Err(_) => {
                eprintln!("WORKER_COUNT={} is not a number; falling back to 3.", raw);
                3
            }
        },
        Err(_) => 3,
    };

    let container = AppContainer::new(worker_count).await?;

    let janitor = container.storage_janitor.clone();
    tokio::spawn(async move { janitor.run().await });

    let server = HttpServer::new(
        container.file_handler,
        container.content_handler,
        container.search_handler,
        container.search_queries_handler,
        container.job_handler,
        container.sse_handler,
        container.chunk_handler,
        container.embedding_handler,
        container.health_handler,
        container.tenants_handler,
        container.background_processor,
        container.auth_repository,
        Some(host),
        Some(port),
    );

    server.run().await?;

    Ok(())
}
