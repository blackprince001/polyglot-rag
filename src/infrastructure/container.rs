use std::{env, sync::Arc};

use crate::{
    application::{
        ports::{DocumentExtractor, EmbeddingProvider, FileStorage, JobQueue},
        services::{DocumentProcessorService, EmbeddingService, SearchService},
        use_cases::{
            CancelJobUseCase, CompleteUploadUseCase, GetFileChunksUseCase, GetFileUseCase,
            GetJobStatusUseCase, ListFilesUseCase, ProcessDocumentUseCase,
            ProcessTextDirectUseCase, ProcessUrlDirectUseCase, ProcessYoutubeDirectUseCase,
            QueueProcessingJobUseCase, RequestUploadUrlUseCase, SearchContentUseCase,
            UploadFileUseCase, UploadWithProcessingUseCase,
        },
    },
    domain::repositories::{
        AssetRepository, AuthRepository, ChunkRepository, EmbeddingRepository, FileRepository,
        JobRepository,
    },
    infrastructure::{
        database::{
            create_connection_pool, get_database_connection,
            repositories::{
                PostgresAssetRepository, PostgresAuthRepository, PostgresChunkRepository,
                PostgresEmbeddingRepository, PostgresFileRepository, PostgresJobRepository,
            },
            run_migrations,
        },
        external_services::{InferenceEmbeddingProvider, document_extractors::ExtractorRegistry},
        file_system::StorageConfig,
        messaging::{BackgroundProcessor, MpscJobQueue},
    },
    presentation::http::handlers::{
        ChunkHandler, ContentHandler, EmbeddingHandler, FileHandler, HealthHandler, JobHandler,
        SearchHandler, SseHandler, TenantsHandler,
    },
};


pub struct AppContainer {
    // Background processing + auth (consumed by the server/middleware)
    pub background_processor: Arc<BackgroundProcessor>,
    pub storage_janitor: Arc<crate::infrastructure::messaging::storage_janitor::StorageJanitor>,
    pub auth_repository: Arc<dyn AuthRepository>,

    // HTTP Handlers
    pub file_handler: Arc<FileHandler>,
    pub content_handler: Arc<ContentHandler>,
    pub search_handler: Arc<SearchHandler>,
    pub job_handler: Arc<JobHandler>,
    pub sse_handler: Arc<SseHandler>,
    pub chunk_handler: Arc<ChunkHandler>,
    pub embedding_handler: Arc<EmbeddingHandler>,
    pub health_handler: Arc<HealthHandler>,
    pub tenants_handler: Arc<TenantsHandler>,
}

impl AppContainer {
    pub async fn new(worker_count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        // Create database connection pool
        let db_pool = create_connection_pool()?;
        let mut conn = get_database_connection()
            .map_err(|e| format!("Failed to create database connection: {}", e))?;
        let _ = run_migrations(&mut conn)
            .map_err(|e| format!("Failed to run Database migrations: {}", e));

        // Create repositories
        let file_repository: Arc<dyn FileRepository> =
            Arc::new(PostgresFileRepository::new(db_pool.clone()));
        let chunk_repository: Arc<dyn ChunkRepository> =
            Arc::new(PostgresChunkRepository::new(db_pool.clone()));
        let embedding_repository: Arc<dyn EmbeddingRepository> =
            Arc::new(PostgresEmbeddingRepository::new(db_pool.clone()));
        let auth_repository: Arc<dyn AuthRepository> =
            Arc::new(PostgresAuthRepository::new(db_pool.clone()));
        let asset_repository: Arc<dyn AssetRepository> =
            Arc::new(PostgresAssetRepository::new(db_pool.clone()));
        let job_repository: Arc<dyn JobRepository> = Arc::new(PostgresJobRepository::new(db_pool));

        // Create external services
        let embedding_provider: Arc<dyn EmbeddingProvider> =
            Arc::new(InferenceEmbeddingProvider::from_env()?);

        let storage_config =
            StorageConfig::from_env().map_err(|e| format!("Storage config error: {}", e))?;
        let file_storage: Arc<dyn FileStorage> = storage_config
            .build()
            .await
            .map_err(|e| format!("Storage backend init failed: {}", e))?;

        // Create document extractor
        let document_extractor: Arc<dyn DocumentExtractor> = Arc::new(
            ExtractorRegistry::with_defaults()
                .map_err(|e| format!("Failed to create document extractor: {}", e))?,
        );

        // Create application services
        let embedding_service = Arc::new(EmbeddingService::new(embedding_provider.clone()));
        let search_service = Arc::new(SearchService::new(
            embedding_provider.clone(),
            embedding_repository.clone(),
            chunk_repository.clone(),
            file_repository.clone(),
            asset_repository.clone(),
        ));

        // Create document processor service
        let document_processor = Arc::new(DocumentProcessorService::new(
            document_extractor.clone(),
            embedding_service.clone(),
            chunk_repository.clone(),
            embedding_repository.clone(),
            file_repository.clone(),
            asset_repository.clone(),
            file_storage.clone(),
        ));

        // Create use cases
        let upload_file_use_case = Arc::new(UploadFileUseCase::new(
            file_repository.clone(),
            file_storage.clone(),
        ));

        let list_files_use_case = Arc::new(ListFilesUseCase::new(file_repository.clone()));

        let process_document_use_case = Arc::new(ProcessDocumentUseCase::new(
            file_repository.clone(),
            document_processor.clone(),
        ));

        let search_content_use_case = Arc::new(SearchContentUseCase::new(search_service.clone()));

        let get_file_use_case = Arc::new(GetFileUseCase::new(file_repository.clone()));

        // Create job queue and background processor
        let (job_queue, job_receiver) = MpscJobQueue::create_pair();
        let job_queue: Arc<dyn JobQueue> = Arc::new(job_queue);
        let job_receiver = Arc::new(job_receiver);

        let background_processor = Arc::new(
            BackgroundProcessor::new(
                job_receiver,
                job_repository.clone(),
                file_repository.clone(),
                document_processor.clone(),
                document_extractor.clone(),
                embedding_service.clone(),
                file_storage.clone(),
                chunk_repository.clone(),
                embedding_repository.clone(),
            )
            .with_worker_count(worker_count),
        );

        // Create async use cases
        let queue_job_use_case = Arc::new(QueueProcessingJobUseCase::new(
            job_repository.clone(),
            job_queue.clone(),
            file_repository.clone(),
        ));

        let upload_with_processing_use_case = Arc::new(UploadWithProcessingUseCase::new(
            upload_file_use_case.clone(),
            queue_job_use_case.clone(),
            file_repository.clone(),
        ));

        let get_job_status_use_case = Arc::new(GetJobStatusUseCase::new(job_repository.clone()));

        let cancel_job_use_case = Arc::new(CancelJobUseCase::new(
            job_repository.clone(),
            job_queue.clone(),
        ));

        let process_url_direct_use_case = Arc::new(ProcessUrlDirectUseCase::new(
            file_repository.clone(),
            queue_job_use_case.clone(),
        ));

        let process_youtube_direct_use_case = Arc::new(ProcessYoutubeDirectUseCase::new(
            file_repository.clone(),
            queue_job_use_case.clone(),
        ));

        let process_text_direct_use_case = Arc::new(ProcessTextDirectUseCase::new(
            file_repository.clone(),
            file_storage.clone(),
            queue_job_use_case.clone(),
        ));

        let request_upload_url_use_case = Arc::new(RequestUploadUrlUseCase::new(
            file_repository.clone(),
            file_storage.clone(),
            storage_config.presigned_upload_ttl_secs(),
        ));
        let complete_upload_use_case = Arc::new(CompleteUploadUseCase::new(
            file_repository.clone(),
            queue_job_use_case.clone(),
        ));

        // Create HTTP handlers
        let file_handler = Arc::new(FileHandler::new(
            upload_file_use_case.clone(),
            upload_with_processing_use_case.clone(),
            list_files_use_case.clone(),
            process_document_use_case.clone(),
            get_file_use_case.clone(),
            request_upload_url_use_case.clone(),
            complete_upload_use_case.clone(),
            file_repository.clone(),
            asset_repository.clone(),
            file_storage.clone(),
            storage_config.presigned_download_ttl_secs(),
        ));

        let search_handler = Arc::new(SearchHandler::new(search_content_use_case.clone()));

        let job_handler = Arc::new(JobHandler::new(
            queue_job_use_case.clone(),
            get_job_status_use_case.clone(),
            cancel_job_use_case.clone(),
        ));

        let sse_handler = Arc::new(SseHandler::new(get_job_status_use_case.clone()));

        let content_handler = Arc::new(ContentHandler::new(
            process_url_direct_use_case.clone(),
            process_youtube_direct_use_case.clone(),
            process_text_direct_use_case.clone(),
        ));

        let get_file_chunks_use_case = Arc::new(GetFileChunksUseCase::new(
            file_repository.clone(),
            chunk_repository.clone(),
            asset_repository.clone(),
        ));
        let chunk_handler = Arc::new(ChunkHandler::new(
            chunk_repository.clone(),
            get_file_chunks_use_case.clone(),
        ));
        let embedding_handler = Arc::new(EmbeddingHandler::new(
            embedding_repository.clone(),
            search_service.clone(),
        ));

        let health_handler = Arc::new(HealthHandler::new(env::var("EMBEDDINGS_SERVICE_URL").ok()));

        let tenants_handler = Arc::new(TenantsHandler::new(auth_repository.clone()));

        let storage_janitor = Arc::new(
            crate::infrastructure::messaging::storage_janitor::StorageJanitor::new(
                file_repository.clone(),
                file_storage.clone(),
                queue_job_use_case.clone(),
                env_u64("JANITOR_INTERVAL_SECS", 300),
                {
                    let upload_ttl = storage_config.presigned_upload_ttl_secs();
                    let default_dangling = (upload_ttl * 2).min(i64::MAX as u64) as i64;
                    env_i64("JANITOR_DANGLING_THRESHOLD_SECS", default_dangling).max(1800)
                },
                env_i64("JANITOR_STALE_PROCESSING_SECS", 1800),
            ),
        );

        Ok(Self {
            background_processor,
            storage_janitor,
            auth_repository,
            file_handler,
            content_handler,
            search_handler,
            job_handler,
            sse_handler,
            chunk_handler,
            embedding_handler,
            health_handler,
            tenants_handler,
        })
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

fn env_i64(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
