use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use reqwest::Client;

use crate::presentation::http::dto::ApiResponse;
use crate::presentation::http::dto::response_dto::{DependencyStatus, HealthResponseDto};

pub struct HealthHandler {
    embedding_service_url: Option<String>,
    http_client: Client,
    probe_timeout: Duration,
}

impl HealthHandler {
    pub fn new(embedding_service_url: Option<String>) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("reqwest client should build with sensible defaults");

        Self {
            embedding_service_url,
            http_client,
            probe_timeout: Duration::from_secs(2),
        }
    }

    pub async fn health(State(handler): State<Arc<Self>>) -> impl IntoResponse {
        let mut dependencies: BTreeMap<String, DependencyStatus> = BTreeMap::new();

        let embed_status = handler
            .probe_embedding_service()
            .await
            .unwrap_or_else(|msg| DependencyStatus {
                status: "down".to_string(),
                message: Some(msg),
            });
        dependencies.insert("embeddings".to_string(), embed_status);

        let overall = if dependencies.values().all(|d| d.status == "up") {
            "healthy"
        } else {
            "degraded"
        };

        let body = HealthResponseDto {
            status: overall.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            dependencies,
        };

        (StatusCode::OK, Json(ApiResponse::success(body)))
    }

    async fn probe_embedding_service(&self) -> Result<DependencyStatus, String> {
        let base = match &self.embedding_service_url {
            Some(s) if !s.trim().is_empty() => s.trim_end_matches('/'),
            _ => return Err("EMBEDDINGS_SERVICE_URL not set".to_string()),
        };

        let url = format!("{}/health", base);
        let started = Instant::now();
        let result = self
            .http_client
            .get(&url)
            .timeout(self.probe_timeout)
            .send()
            .await;

        match result {
            Ok(resp) if resp.status().is_success() => Ok(DependencyStatus {
                status: "up".to_string(),
                message: Some(format!("{} ms", started.elapsed().as_millis())),
            }),
            Ok(resp) => Err(format!("HTTP {}", resp.status())),
            Err(e) if e.is_timeout() => Err("timeout (2s)".to_string()),
            Err(e) => Err(format!("unreachable: {}", e)),
        }
    }
}
