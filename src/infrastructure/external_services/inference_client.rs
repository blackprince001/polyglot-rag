use async_trait::async_trait;
use pgvector::Vector;
use reqwest::{Client, Error as ReqwestError};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::application::ports::embedding_provider::{
    BatchEmbeddingRequest, BatchEmbeddingResponse, EmbeddingProvider, EmbeddingProviderError,
    EmbeddingRequest, EmbeddingResponse,
};

// TEI API request/response structures based on OpenAPI spec
#[derive(Serialize)]
pub struct TeiEmbedRequest {
    pub inputs: TeiInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalize: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncate: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum TeiInput {
    Single(String),
    Multiple(Vec<String>),
}

// TEI response is just an array of arrays of floats
pub type TeiEmbedResponse = Vec<Vec<f32>>;

#[derive(Deserialize)]
pub struct TeiErrorResponse {
    pub error: String,
    pub error_type: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddingsClientConfig {
    pub service_url: String,
    pub max_retries: u32,
    pub timeout_secs: u64,
    pub backoff_factor: f64,
}

impl Default for EmbeddingsClientConfig {
    fn default() -> Self {
        let service_url = env::var("EMBEDDINGS_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        Self {
            service_url,
            max_retries: 3,
            timeout_secs: 30,
            backoff_factor: 1.5,
        }
    }
}

#[derive(Debug)]
pub enum EmbeddingsError {
    RequestError(String),
    ParseError(String),
    // MaxRetriesExceeded(String),
    ApiError(String),
}

#[derive(Debug, Clone)]
pub struct InferenceClient {
    client: Client,
    config: EmbeddingsClientConfig,
}

impl InferenceClient {
    pub fn new(config: EmbeddingsClientConfig) -> Result<Self, ReqwestError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()?;

        Ok(Self { client, config })
    }

    pub fn from_env() -> Result<Self, ReqwestError> {
        Self::new(EmbeddingsClientConfig::default())
    }

    pub async fn get_embedding(&self, text: &str) -> Result<TeiEmbedResponse, EmbeddingsError> {
        let request = TeiEmbedRequest {
            inputs: TeiInput::Single(text.to_string()),
            normalize: Some(true),
            truncate: Some(false),
            truncation_direction: None,
            prompt_name: None,
            dimensions: None,
        };

        self.send_embed_request(request).await
    }

    pub async fn get_embeddings(
        &self,
        texts: &Vec<String>,
    ) -> Result<TeiEmbedResponse, EmbeddingsError> {
        let request = TeiEmbedRequest {
            inputs: TeiInput::Multiple(texts.to_vec()),
            normalize: Some(true),
            truncate: Some(false),
            truncation_direction: None,
            prompt_name: None,
            dimensions: None,
        };

        self.send_embed_request(request).await
    }

    async fn send_embed_request(
        &self,
        request: TeiEmbedRequest,
    ) -> Result<TeiEmbedResponse, EmbeddingsError> {
        let mut attempts = 0;

        loop {
            attempts += 1;

            let result = self.execute_embed_request(&request).await;

            match result {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if attempts > self.config.max_retries {
                        return Err(e);
                    }

                    let backoff_time = Duration::from_millis(
                        (self.config.backoff_factor.powi(attempts as i32 - 1) * 1000.0) as u64,
                    );

                    tokio::time::sleep(backoff_time).await;
                }
            }
        }
    }

    async fn execute_embed_request(
        &self,
        request: &TeiEmbedRequest,
    ) -> Result<TeiEmbedResponse, EmbeddingsError> {
        let url = format!("{}/embed", self.config.service_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(request)
            .send()
            .await
            .map_err(|e| EmbeddingsError::RequestError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            // Try to parse error response
            match response.json::<TeiErrorResponse>().await {
                Ok(error_response) => {
                    return Err(EmbeddingsError::ApiError(format!(
                        "TEI API error: {} (type: {})",
                        error_response.error, error_response.error_type
                    )));
                }
                Err(_) => {
                    return Err(EmbeddingsError::ApiError(format!("HTTP error: {}", status)));
                }
            }
        }

        let embeddings = response
            .json::<TeiEmbedResponse>()
            .await
            .map_err(|e| EmbeddingsError::ParseError(e.to_string()))?;

        Ok(embeddings)
    }
}

// Adapter to implement the EmbeddingProvider trait
pub struct InferenceEmbeddingProvider {
    client: InferenceClient,
}

impl InferenceEmbeddingProvider {
    pub fn from_env() -> Result<Self, ReqwestError> {
        let client = InferenceClient::from_env()?;
        Ok(Self { client })
    }

    // Helper to convert f32 Vec to pgvector::Vector
    fn to_pgvector(embedding: Vec<f32>) -> Vector {
        Vector::from(embedding)
    }
}

#[async_trait]
impl EmbeddingProvider for InferenceEmbeddingProvider {
    async fn generate_embedding(
        &self,
        request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, EmbeddingProviderError> {
        let response = self
            .client
            .get_embedding(&request.text)
            .await
            .map_err(|e| match e {
                EmbeddingsError::RequestError(msg) => EmbeddingProviderError::NetworkError(msg),
                EmbeddingsError::ParseError(msg) => EmbeddingProviderError::ApiError(msg),
                EmbeddingsError::ApiError(msg) => EmbeddingProviderError::ApiError(msg),
                // EmbeddingsError::MaxRetriesExceeded(_) => {
                //     EmbeddingProviderError::ServiceUnavailable
                // }
            })?;

        if response.is_empty() {
            return Err(EmbeddingProviderError::ApiError(
                "No embeddings returned".to_string(),
            ));
        }

        Ok(EmbeddingResponse {
            embedding: Self::to_pgvector(response[0].clone()),
        })
    }

    async fn generate_embeddings(
        &self,
        request: BatchEmbeddingRequest,
    ) -> Result<BatchEmbeddingResponse, EmbeddingProviderError> {
        let response = self
            .client
            .get_embeddings(&request.texts)
            .await
            .map_err(|e| match e {
                EmbeddingsError::RequestError(msg) => EmbeddingProviderError::NetworkError(msg),
                EmbeddingsError::ParseError(msg) => EmbeddingProviderError::ApiError(msg),
                EmbeddingsError::ApiError(msg) => EmbeddingProviderError::ApiError(msg),
            })?;

        let embeddings = response.into_iter().map(Self::to_pgvector).collect();

        Ok(BatchEmbeddingResponse {
            embeddings,
            model_name: request
                .model_name
                .unwrap_or_else(|| "qwen-embedding".to_string()),
            model_version: request.model_version,
        })
    }

    fn model_info(&self) -> (String, Option<String>) {
        (
            "Qwen/Qwen3-Embedding-0.6B".to_string(),
            Some("0.6B".to_string()),
        )
    }
}
