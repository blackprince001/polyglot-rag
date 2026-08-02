use std::sync::Arc;

use crate::application::ports::embedding_provider::{BatchEmbeddingRequest, EmbeddingProvider};
use crate::application::ports::progress_sink::ProgressSink;
use crate::domain::entities::{ContentChunk, Embedding};

#[derive(Debug)]
pub enum EmbeddingServiceError {
    ProviderError(String),
}

impl std::fmt::Display for EmbeddingServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingServiceError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
        }
    }
}

impl std::error::Error for EmbeddingServiceError {}

pub struct EmbeddingService {
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl EmbeddingService {
    const BATCH_SIZE: usize = 10;

    pub fn new(embedding_provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self { embedding_provider }
    }

    /// Embedding dominates a document's wall-clock, so each completed batch
    /// reports through `progress` as a fraction of this phase.
    pub async fn generate_embeddings_for_chunks(
        &self,
        chunks: &[ContentChunk],
        progress: Option<&dyn ProgressSink>,
    ) -> Result<Vec<Embedding>, EmbeddingServiceError> {
        let (model_name, model_version) = self.embedding_provider.model_info();
        let mut embeddings = Vec::with_capacity(chunks.len());
        let total_batches = chunks.len().div_ceil(Self::BATCH_SIZE).max(1);

        for (batch_index, batch) in chunks.chunks(Self::BATCH_SIZE).enumerate() {
            let texts: Vec<String> = batch.iter().map(|c| c.chunk_text().to_string()).collect();

            let request = BatchEmbeddingRequest {
                texts,
                model_name: Some(model_name.clone()),
                model_version: model_version.clone(),
            };

            let response = self
                .embedding_provider
                .generate_embeddings(request)
                .await
                .map_err(|e| EmbeddingServiceError::ProviderError(e.to_string()))?;

            for (chunk, vector) in batch.iter().zip(response.embeddings.iter()) {
                embeddings.push(Embedding::new(
                    chunk.id(),
                    response.model_name.clone(),
                    response.model_version.clone(),
                    None,
                    vector.clone(),
                ));
            }

            if let Some(sink) = progress {
                let done = batch_index + 1;
                sink.report(
                    done as f32 / total_batches as f32,
                    Some(format!(
                        "Embedded {} of {} chunks",
                        embeddings.len(),
                        chunks.len()
                    )),
                )
                .await;
            }
        }

        Ok(embeddings)
    }
}
