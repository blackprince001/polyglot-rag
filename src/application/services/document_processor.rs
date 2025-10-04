use std::sync::Arc;
use uuid::Uuid;

use crate::application::ports::{
    DocumentExtractor, EmbeddingProvider,
    document_extractor::{ExtractedContent, ExtractionOptions},
    embedding_provider::BatchEmbeddingRequest,
};
use crate::domain::entities::{ContentChunk, Embedding, File};
use crate::domain::repositories::{ChunkRepository, EmbeddingRepository, FileRepository};

#[derive(Debug, Clone)]
pub struct ChunkingConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub max_chunks_per_document: Option<usize>,
    pub min_chunk_size: usize,
}

#[derive(Debug)]
pub enum DocumentProcessingError {
    ExtractionError(String),
    EmbeddingError(String),
    RepositoryError(String),
}

impl std::fmt::Display for DocumentProcessingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentProcessingError::ExtractionError(msg) => write!(f, "Extraction error: {}", msg),
            DocumentProcessingError::EmbeddingError(msg) => write!(f, "Embedding error: {}", msg),
            DocumentProcessingError::RepositoryError(msg) => write!(f, "Repository error: {}", msg),
        }
    }
}

impl std::error::Error for DocumentProcessingError {}

pub struct DocumentProcessorService {
    document_extractor: Arc<dyn DocumentExtractor>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    chunk_repository: Arc<dyn ChunkRepository>,
    embedding_repository: Arc<dyn EmbeddingRepository>,
    file_repository: Arc<dyn FileRepository>,
    chunking_config: ChunkingConfig,
}

impl ChunkingConfig {
    /// Creates a new ChunkingConfig from environment variables with sensible defaults
    pub fn from_env() -> Self {
        Self {
            chunk_size: std::env::var("CHUNK_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(578),
            chunk_overlap: std::env::var("CHUNK_OVERLAP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(102),
            max_chunks_per_document: std::env::var("MAX_CHUNKS_PER_DOCUMENT")
                .ok()
                .and_then(|s| s.parse().ok()),
            min_chunk_size: std::env::var("MIN_CHUNK_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }

    /// Creates a new ChunkingConfig with custom values
    pub fn new(
        chunk_size: usize,
        chunk_overlap: usize,
        max_chunks_per_document: Option<usize>,
        min_chunk_size: usize,
    ) -> Self {
        Self {
            chunk_size,
            chunk_overlap,
            max_chunks_per_document,
            min_chunk_size,
        }
    }

    /// Validates the chunking configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.chunk_size == 0 {
            return Err("Chunk size must be greater than 0".to_string());
        }
        if self.chunk_overlap >= self.chunk_size {
            return Err("Chunk overlap must be less than chunk size".to_string());
        }
        if self.min_chunk_size > self.chunk_size {
            return Err("Minimum chunk size cannot be greater than chunk size".to_string());
        }
        Ok(())
    }
}

impl DocumentProcessorService {
    pub fn new(
        document_extractor: Arc<dyn DocumentExtractor>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        chunk_repository: Arc<dyn ChunkRepository>,
        embedding_repository: Arc<dyn EmbeddingRepository>,
        file_repository: Arc<dyn FileRepository>,
    ) -> Self {
        let chunking_config = ChunkingConfig::from_env();
        Self::new_with_config(
            document_extractor,
            embedding_provider,
            chunk_repository,
            embedding_repository,
            file_repository,
            chunking_config,
        )
    }

    pub fn new_with_config(
        document_extractor: Arc<dyn DocumentExtractor>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
        chunk_repository: Arc<dyn ChunkRepository>,
        embedding_repository: Arc<dyn EmbeddingRepository>,
        file_repository: Arc<dyn FileRepository>,
        chunking_config: ChunkingConfig,
    ) -> Self {
        // Validate configuration
        if let Err(e) = chunking_config.validate() {
            eprintln!("Warning: Invalid chunking configuration: {}", e);
            eprintln!("Using default configuration instead");
            let default_config = ChunkingConfig::new(578, 102, None, 10);
            Self {
                document_extractor,
                embedding_provider,
                chunk_repository,
                embedding_repository,
                file_repository,
                chunking_config: default_config,
            }
        } else {
            Self {
                document_extractor,
                embedding_provider,
                chunk_repository,
                embedding_repository,
                file_repository,
                chunking_config,
            }
        }
    }

    /// Sanitizes text to ensure it's safe for database storage
    /// Removes null bytes and other problematic characters that can cause UTF-8 encoding issues
    fn sanitize_text_for_database(text: &str) -> String {
        text.chars()
            .filter(|c| *c != '\0') // Remove null bytes
            .collect::<String>()
    }

    /// Returns the current chunking configuration
    pub fn chunking_config(&self) -> &ChunkingConfig {
        &self.chunking_config
    }

    pub async fn process_file(
        &self,
        file: &File,
        extraction_options: ExtractionOptions,
    ) -> Result<(i32, i32), DocumentProcessingError> {
        println!(
            "Processing file: {} with chunking config: {:?}",
            file.file_name(),
            self.chunking_config
        );

        let extracted_content = self
            .extract_text_from_file(file, extraction_options)
            .await?;

        let chunks = self.create_chunks(file.id(), &extracted_content.text)?;

        println!(
            "Created {} chunks for file: {}",
            chunks.len(),
            file.file_name()
        );

        match self.file_repository.find_by_id(file.id()).await {
            Ok(Some(_verified_file)) => {}
            Ok(None) => {
                return Err(DocumentProcessingError::RepositoryError(format!(
                    "File {} disappeared from database before saving chunks",
                    file.id()
                )));
            }
            Err(e) => {
                return Err(DocumentProcessingError::RepositoryError(format!(
                    "Failed to verify file exists before saving chunks: {}",
                    e
                )));
            }
        }

        // Save chunks and get their database-generated IDs
        let chunk_ids = self
            .chunk_repository
            .save_batch(&chunks)
            .await
            .map_err(|e| DocumentProcessingError::RepositoryError(e.to_string()))?;

        // Update chunks with their database IDs
        let mut chunks_with_ids = Vec::new();
        for (chunk, chunk_id) in chunks.iter().zip(chunk_ids.iter()) {
            let chunk_with_id = ContentChunk::with_id(
                *chunk_id,
                chunk.file_id(),
                chunk.chunk_text().to_string(),
                chunk.chunk_index(),
                chunk.token_count(),
                chunk.page_number(),
                chunk.section_path().map(|s| s.to_string()),
                chunk.created_at(),
            );
            chunks_with_ids.push(chunk_with_id);
        }

        let embeddings = self
            .generate_embeddings_for_chunks(&chunks_with_ids)
            .await?;

        self.embedding_repository
            .save_batch(&embeddings)
            .await
            .map_err(|e| DocumentProcessingError::RepositoryError(e.to_string()))?;

        Ok((chunks.len() as i32, embeddings.len() as i32))
    }

    async fn extract_text_from_file(
        &self,
        file: &File,
        extraction_options: ExtractionOptions,
    ) -> Result<ExtractedContent, DocumentProcessingError> {
        self.document_extractor
            .extract_text(file, extraction_options)
            .await
            .map_err(|e| DocumentProcessingError::ExtractionError(e.to_string()))
    }

    fn create_chunks(
        &self,
        file_id: Uuid,
        text: &str,
    ) -> Result<Vec<ContentChunk>, DocumentProcessingError> {
        let mut chunks = Vec::new();

        // Additional safety check: sanitize text to ensure it's valid UTF-8 for database storage
        let sanitized_text = Self::sanitize_text_for_database(text);

        let words: Vec<&str> = sanitized_text.split_whitespace().collect();

        if words.is_empty() {
            return Ok(chunks);
        }

        let mut start = 0;
        let mut chunk_index = 0;

        while start < words.len() {
            // Check if we've reached the maximum number of chunks
            if let Some(max_chunks) = self.chunking_config.max_chunks_per_document {
                if chunks.len() >= max_chunks {
                    eprintln!(
                        "Warning: Reached maximum chunks limit ({}) for document. Stopping chunking.",
                        max_chunks
                    );
                    break;
                }
            }

            // Calculate end position for this chunk
            let end = std::cmp::min(start + self.chunking_config.chunk_size, words.len());

            // Create chunk text
            let chunk_text = words[start..end].join(" ");

            // Skip empty or very small chunks
            if chunk_text.trim().len() < self.chunking_config.min_chunk_size {
                break;
            }

            // Create chunk entity
            let chunk = ContentChunk::new(
                file_id,
                chunk_text,
                chunk_index,
                Some(end as i32 - start as i32), // Approximate token count
                None,                            // Page number - could be extracted from metadata
                None, // Section path - could be extracted from document structure
            );

            chunks.push(chunk);
            chunk_index += 1;

            // Move start position with overlap
            start = if end >= words.len() {
                break;
            } else {
                std::cmp::max(
                    start + self.chunking_config.chunk_size - self.chunking_config.chunk_overlap,
                    start + 1,
                )
            };
        }

        Ok(chunks)
    }

    async fn generate_embeddings_for_chunks(
        &self,
        chunks: &[ContentChunk],
    ) -> Result<Vec<Embedding>, DocumentProcessingError> {
        let mut embeddings = Vec::new();
        let (model_name, model_version) = self.embedding_provider.model_info();

        const BATCH_SIZE: usize = 10;

        for chunk_batch in chunks.chunks(BATCH_SIZE) {
            let texts: Vec<String> = chunk_batch
                .iter()
                .map(|chunk| chunk.chunk_text().to_string())
                .collect();

            let batch_request = BatchEmbeddingRequest {
                texts,
                model_name: Some(model_name.clone()),
                model_version: model_version.clone(),
            };

            let batch_response = self
                .embedding_provider
                .generate_embeddings(batch_request)
                .await
                .map_err(|e| DocumentProcessingError::EmbeddingError(e.to_string()))?;

            for (chunk, embedding_vector) in
                chunk_batch.iter().zip(batch_response.embeddings.iter())
            {
                let embedding = Embedding::new(
                    chunk.id(),
                    batch_response.model_name.clone(),
                    batch_response.model_version.clone(),
                    None,
                    embedding_vector.clone(),
                );

                embeddings.push(embedding);
            }
        }

        Ok(embeddings)
    }
}
