use async_trait::async_trait;
use pgvector::Vector;

#[derive(Debug)]
pub enum EmbeddingProviderError {
    NetworkError(String),
    ApiError(String),
}

impl std::fmt::Display for EmbeddingProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingProviderError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            EmbeddingProviderError::ApiError(msg) => write!(f, "API error: {}", msg),
        }
    }
}

impl std::error::Error for EmbeddingProviderError {}

#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    pub embedding: Vector,
}

#[derive(Debug, Clone)]
pub struct BatchEmbeddingRequest {
    pub texts: Vec<String>,
    pub model_name: Option<String>,
    pub model_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BatchEmbeddingResponse {
    pub embeddings: Vec<Vector>,
    pub model_name: String,
    pub model_version: Option<String>,
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn generate_embedding(
        &self,
        request: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, EmbeddingProviderError>;

    async fn generate_embeddings(
        &self,
        request: BatchEmbeddingRequest,
    ) -> Result<BatchEmbeddingResponse, EmbeddingProviderError>;

    fn model_info(&self) -> (String, Option<String>);
}
