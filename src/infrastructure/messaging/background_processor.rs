use std::sync::Arc;

use tracing::{error, info, warn};
use uuid::Uuid;

use crate::application::ports::document_extractor::DocumentExtractor;
use crate::application::ports::document_extractor::ExtractionOptions;
use crate::application::ports::file_storage::FileStorage;
use crate::application::services::{DocumentProcessorService, EmbeddingService};
use crate::domain::entities::processing_job::{JobResult, JobType, ProcessingJob};
use crate::domain::entities::{ContentChunk, File};
use crate::domain::repositories::{
    ChunkRepository, EmbeddingRepository, FileRepository, JobRepository,
};
use crate::infrastructure::external_services::semantic_chunking::{
    RTSplitter, RecursiveTextSplitter,
};
use crate::infrastructure::messaging::MpscJobQueueReceiver;

pub struct BackgroundProcessor {
    job_receiver: Arc<MpscJobQueueReceiver>,
    job_repository: Arc<dyn JobRepository>,
    file_repository: Arc<dyn FileRepository>,
    document_processor: Arc<DocumentProcessorService>,
    document_extractor: Arc<dyn DocumentExtractor>,
    embedding_service: Arc<EmbeddingService>,
    file_storage: Arc<dyn FileStorage>,
    chunk_repository: Arc<dyn ChunkRepository>,
    embedding_repository: Arc<dyn EmbeddingRepository>,
    text_splitter: RTSplitter,
    worker_count: usize,
}

impl BackgroundProcessor {
    pub fn new(
        job_receiver: Arc<MpscJobQueueReceiver>,
        job_repository: Arc<dyn JobRepository>,
        file_repository: Arc<dyn FileRepository>,
        document_processor: Arc<DocumentProcessorService>,
        document_extractor: Arc<dyn DocumentExtractor>,
        embedding_service: Arc<EmbeddingService>,
        file_storage: Arc<dyn FileStorage>,
        chunk_repository: Arc<dyn ChunkRepository>,
        embedding_repository: Arc<dyn EmbeddingRepository>,
    ) -> Self {
        Self {
            job_receiver,
            job_repository,
            file_repository,
            document_processor,
            document_extractor,
            embedding_service,
            file_storage,
            chunk_repository,
            embedding_repository,
            text_splitter: RTSplitter::default(),
            worker_count: 3, // Default worker count
        }
    }

    pub fn with_worker_count(mut self, count: usize) -> Self {
        self.worker_count = count.max(1); // At least 1 worker
        self
    }

    pub async fn start(&self) {
        info!(workers = self.worker_count, "starting background processor");

        // Spawn multiple worker tasks
        let mut handles = Vec::new();

        for worker_id in 0..self.worker_count {
            let processor = self.clone_for_worker();
            let handle = tokio::spawn(async move {
                processor.worker_loop(worker_id).await;
            });
            handles.push(handle);
        }

        // Wait for all workers to complete (they shouldn't unless there's an error)
        for (i, handle) in handles.into_iter().enumerate() {
            if let Err(e) = handle.await {
                error!(worker_id = i, error = %e, "worker panicked");
            }
        }

        info!("background processor stopped");
    }

    async fn worker_loop(&self, worker_id: usize) {
        info!(worker_id, "worker started");

        loop {
            match self.job_receiver.recv().await {
                Some(v) => {
                    info!(worker_id, job_id = %v.id(), "processing job");
                    self.process_job(v).await;
                }
                None => {
                    info!(worker_id, "channel closed, worker stopping");
                    break;
                }
            }
        }

        info!(worker_id, "worker stopped");
    }

    async fn process_job(&self, mut job: ProcessingJob) {
        let job_id = job.id();
        let tenant = job.tenant_id();
        let file_id = job.file_id();
        let start_time = std::time::Instant::now();

        // Job status → processing.
        if let Err(e) = job.start_processing() {
            error!(%job_id, error = %e, "failed to start job");
            return;
        }
        if let Err(e) = self.job_repository.update(&job).await {
            error!(%job_id, error = %e, "failed to persist job 'processing' status");
            return;
        }

        self.mark_file_processing(tenant, file_id).await;

        let result = match job.job_type().clone() {
            JobType::FileProcessing => self.process_file_job(&mut job).await,
            JobType::UrlExtraction { url } => self.process_url_extraction_job(&mut job, &url).await,
            JobType::YoutubeExtraction { url } => {
                self.process_youtube_extraction_job(&mut job, &url).await
            }
        };

        match result {
            Ok(job_result) => {
                if let Err(e) = job.complete_processing(job_result) {
                    error!(%job_id, error = %e, "failed to mark job completed");
                } else {
                    info!(
                        %job_id,
                        elapsed_ms = start_time.elapsed().as_millis() as u64,
                        "job completed"
                    );
                }
                self.mark_file_completed(tenant, file_id).await;
            }
            Err(cause) => {
                if let Err(e) = job.fail_processing(cause.clone()) {
                    error!(%job_id, error = %e, "failed to mark job failed");
                } else {
                    warn!(%job_id, cause = %cause, "job failed");
                }
                self.mark_file_failed(tenant, file_id, cause).await;
            }
        }

        // Persist the terminal job state.
        if let Err(e) = self.job_repository.update(&job).await {
            error!(%job_id, error = %e, "failed to persist final job state");
        }
    }

    /// Load a file for a status update, logging (not failing) when it's missing.
    async fn load_file_for_status(&self, tenant: Uuid, file_id: Uuid) -> Option<File> {
        match self.file_repository.find_by_id(tenant, file_id).await {
            Ok(Some(file)) => Some(file),
            Ok(None) => {
                warn!(%file_id, "file not found while updating status");
                None
            }
            Err(e) => {
                error!(%file_id, error = %e, "failed to load file for status update");
                None
            }
        }
    }

    async fn mark_file_processing(&self, tenant: Uuid, file_id: Uuid) {
        let Some(mut file) = self.load_file_for_status(tenant, file_id).await else {
            return;
        };
        if let Err(e) = file.start_processing() {
            warn!(%file_id, error = %e, "could not transition file to processing");
            return;
        }
        if let Err(e) = self.file_repository.update(tenant, &file).await {
            error!(%file_id, error = %e, "failed to persist file 'processing' status");
        }
    }

    async fn mark_file_completed(&self, tenant: Uuid, file_id: Uuid) {
        let Some(mut file) = self.load_file_for_status(tenant, file_id).await else {
            return;
        };
        if let Err(e) = file.complete_processing() {
            warn!(%file_id, error = %e, "could not transition file to completed");
            return;
        }
        if let Err(e) = self.file_repository.update(tenant, &file).await {
            error!(%file_id, error = %e, "failed to persist file 'completed' status");
        }
    }

    async fn mark_file_failed(&self, tenant: Uuid, file_id: Uuid, cause: String) {
        let Some(mut file) = self.load_file_for_status(tenant, file_id).await else {
            return;
        };
        if let Err(e) = file.fail_processing(cause) {
            warn!(%file_id, error = %e, "could not transition file to failed");
            return;
        }
        if let Err(e) = self.file_repository.update(tenant, &file).await {
            error!(%file_id, error = %e, "failed to persist file 'failed' status");
        }
    }

    async fn process_file_job(&self, job: &mut ProcessingJob) -> Result<JobResult, String> {
        let tenant = job.tenant_id();

        let _ = job.update_progress(0.2, Some("Processing document...".to_string()));
        let _ = self.job_repository.update(job).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let file = self
            .file_repository
            .find_by_id(tenant, job.file_id())
            .await
            .map_err(|e| format!("Failed to find file: {}", e))?
            .ok_or_else(|| format!("File not found in database: {}", job.file_id()))?;

        let outcome = self
            .document_processor
            .process_file(tenant, &file, ExtractionOptions::default())
            .await
            .map_err(|e| format!("Document processing failed: {}", e))?;

        Ok(JobResult {
            chunks_created: outcome.chunks_created,
            embeddings_created: outcome.embeddings_created,
            assets_created: outcome.assets_created,
            processing_time_ms: 0,
            extracted_text_length: 0,
        })
    }

    async fn process_url_extraction_job(
        &self,
        job: &mut ProcessingJob,
        url: &str,
    ) -> Result<JobResult, String> {
        let tenant = job.tenant_id();
        let _ = job.update_progress(0.1, Some("Extracting content from URL...".to_string()));
        let _ = self.job_repository.update(job).await;

        let extracted = self
            .document_extractor
            .extract_text_from_bytes(
                url.as_bytes(),
                "text/html",
                ExtractionOptions {
                    extract_metadata: true,
                    max_pages: None,
                },
            )
            .await
            .map_err(|e| format!("URL extraction failed: {}", e))?;

        self.ingest_text(job, tenant, &extracted.full_text).await
    }

    async fn process_youtube_extraction_job(
        &self,
        job: &mut ProcessingJob,
        url: &str,
    ) -> Result<JobResult, String> {
        let tenant = job.tenant_id();
        let _ = job.update_progress(0.1, Some("Fetching transcript...".to_string()));
        let _ = self.job_repository.update(job).await;

        let extracted = self
            .document_extractor
            .extract_text_from_bytes(
                url.as_bytes(),
                "text/youtube-url",
                ExtractionOptions {
                    extract_metadata: true,
                    max_pages: None,
                },
            )
            .await
            .map_err(|e| format!("YouTube extraction failed: {}", e))?;

        self.ingest_text(job, tenant, &extracted.full_text).await
    }

    /// Shared chunk → embed → persist pipeline for text-only sources (URL and
    /// transcript jobs). Returns the job tally; file/job status is handled by
    /// the caller chain in `process_job`.
    async fn ingest_text(
        &self,
        job: &mut ProcessingJob,
        tenant: Uuid,
        text: &str,
    ) -> Result<JobResult, String> {
        let _ = job.update_progress(0.3, Some("Creating chunks...".to_string()));
        let _ = self.job_repository.update(job).await;

        let chunks = self.create_chunks_from_text(job.file_id(), text)?;

        let chunk_ids = self
            .chunk_repository
            .save_batch(tenant, &chunks)
            .await
            .map_err(|e| format!("Failed to save chunks: {}", e))?;

        let mut chunks_with_ids = Vec::new();
        for (chunk, chunk_id) in chunks.iter().zip(chunk_ids.iter()) {
            chunks_with_ids.push(ContentChunk::with_id(
                *chunk_id,
                chunk.file_id(),
                chunk.chunk_text().to_string(),
                chunk.chunk_index(),
                chunk.token_count(),
                chunk.page_number(),
                chunk.section_path().map(|s| s.to_string()),
                chunk.created_at(),
            ));
        }

        let _ = job.update_progress(0.6, Some("Generating embeddings...".to_string()));
        let _ = self.job_repository.update(job).await;

        let embeddings = self
            .embedding_service
            .generate_embeddings_for_chunks(&chunks_with_ids)
            .await
            .map_err(|e| format!("Embedding generation failed: {}", e))?;

        self.embedding_repository
            .save_batch(tenant, &embeddings)
            .await
            .map_err(|e| format!("Failed to save embeddings: {}", e))?;

        Ok(JobResult {
            chunks_created: chunks_with_ids.len() as i32,
            embeddings_created: embeddings.len() as i32,
            assets_created: 0,
            processing_time_ms: 0,
            extracted_text_length: text.len(),
        })
    }

    fn create_chunks_from_text(
        &self,
        file_id: uuid::Uuid,
        text: &str,
    ) -> Result<Vec<crate::domain::entities::ContentChunk>, String> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Use RTSplitter with a reasonable chunk size (characters, not words)
        let max_chunk_size = 2000; // characters - good balance for embeddings
        let chunk_texts = self.text_splitter.split_text(text, max_chunk_size);

        let mut chunks = Vec::new();
        for (index, chunk_text) in chunk_texts.into_iter().enumerate() {
            if chunk_text.trim().len() < 10 {
                continue; // Skip very small chunks
            }

            let word_count = chunk_text.split_whitespace().count() as i32;

            let chunk = crate::domain::entities::ContentChunk::new(
                file_id,
                chunk_text,
                index as i32,
                Some(word_count),
                None, // page_number - not applicable for text extraction
                None, // section_path - could be enhanced later
            );

            chunks.push(chunk);
        }

        Ok(chunks)
    }

    fn clone_for_worker(&self) -> Self {
        Self {
            job_receiver: self.job_receiver.clone(),
            job_repository: self.job_repository.clone(),
            file_repository: self.file_repository.clone(),
            document_processor: self.document_processor.clone(),
            document_extractor: self.document_extractor.clone(),
            embedding_service: self.embedding_service.clone(),
            file_storage: self.file_storage.clone(),
            chunk_repository: self.chunk_repository.clone(),
            embedding_repository: self.embedding_repository.clone(),
            text_splitter: self.text_splitter.clone(),
            worker_count: self.worker_count,
        }
    }
}
