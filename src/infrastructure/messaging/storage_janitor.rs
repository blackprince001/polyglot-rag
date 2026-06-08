use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::application::ports::FileStorage;
use crate::application::use_cases::queue_processing_job::{
    QueueJobRequest, QueueProcessingJobUseCase,
};
use crate::domain::entities::processing_job::JobType;
use crate::domain::repositories::FileRepository;
use crate::domain::value_objects::ProcessingStatus;

#[derive(Debug, Default, Clone)]
pub struct JanitorSummary {
    pub swept_dangling: usize,
    pub requeued_stale: usize,
    pub errors: usize,
}

pub struct StorageJanitor {
    file_repository: Arc<dyn FileRepository>,
    file_storage: Arc<dyn FileStorage>,
    queue_job_use_case: Arc<QueueProcessingJobUseCase>,
    /// How often the janitor wakes up. Defaults to 5 min via env.
    interval_secs: u64,
    /// Files in `Pending` for longer than this are treated as dangling
    /// presigned-upload intent. Defaults to 30 min via env (≥ 2× the
    /// presigned upload TTL).
    dangling_intent_threshold_secs: i64,
    /// Files in `Processing` for longer than this are treated as stuck jobs
    /// and re-enqueued. Defaults to 30 min.
    stale_processing_threshold_secs: i64,
}

impl StorageJanitor {
    pub fn new(
        file_repository: Arc<dyn FileRepository>,
        file_storage: Arc<dyn FileStorage>,
        queue_job_use_case: Arc<QueueProcessingJobUseCase>,
        interval_secs: u64,
        dangling_intent_threshold_secs: i64,
        stale_processing_threshold_secs: i64,
    ) -> Self {
        Self {
            file_repository,
            file_storage,
            queue_job_use_case,
            interval_secs,
            dangling_intent_threshold_secs,
            stale_processing_threshold_secs,
        }
    }

    /// Long-running task. Spawn this once at startup; the function returns
    /// only on cancellation (the future dropped) or unrecoverable scheduler
    /// error. Each tick runs the three sweeps back-to-back.
    pub async fn run(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(self.interval_secs));
        // Skip the first immediate tick; we want a real delay between sweeps
        // at startup so other services have a moment to come up.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            let summary = self.sweep_once().await;
            if summary.errors > 0 || summary.swept_dangling > 0 || summary.requeued_stale > 0 {
                eprintln!(
                    "[janitor] tick summary: swept_dangling={} requeued_stale={} errors={}",
                    summary.swept_dangling, summary.requeued_stale, summary.errors
                );
            }
        }
    }

    /// One pass: dangling intent + stale processing. Exposed for tests and
    /// for ad-hoc `/admin/janitor` triggers if we ever add one.
    pub async fn sweep_once(&self) -> JanitorSummary {
        let mut summary = JanitorSummary::default();
        summary.swept_dangling = self.sweep_dangling_intent().await;
        summary.requeued_stale = self.sweep_stale_processing().await;
        summary
    }

    async fn sweep_dangling_intent(&self) -> usize {
        let stale = match self
            .file_repository
            .find_stale_for_janitor(
                self.dangling_intent_threshold_secs,
                &[ProcessingStatus::Pending],
            )
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!("[janitor] find_stale_for_janitor (pending) failed: {}", e);
                return 0;
            }
        };

        let mut swept = 0;
        for (file, tenant_id) in stale {
            let file_id = file.id();
            // Delete the storage object first; the DB row cleanup is the
            // best-effort last step so we don't leave an orphan reference.
            if let Err(e) = self.file_storage.delete(tenant_id, file_id).await {
                eprintln!(
                    "[janitor] dangling intent storage delete failed for {} (tenant={}): {}",
                    file_id, tenant_id, e
                );
                continue;
            }
            if let Err(e) = self.file_repository.delete(tenant_id, file_id).await {
                eprintln!(
                    "[janitor] dangling intent db delete failed for {} (tenant={}): {}",
                    file_id, tenant_id, e
                );
                continue;
            }
            swept += 1;
            let _ = tenant_id; // suppress unused warning under some configs
        }
        swept
    }

    async fn sweep_stale_processing(&self) -> usize {
        let stale = match self
            .file_repository
            .find_stale_for_janitor(
                self.stale_processing_threshold_secs,
                &[ProcessingStatus::Processing],
            )
            .await
        {
            Ok(rows) => rows,
            Err(e) => {
                eprintln!(
                    "[janitor] find_stale_for_janitor (processing) failed: {}",
                    e
                );
                return 0;
            }
        };

        let mut requeued = 0;
        for (file, tenant_id) in stale {
            let file_id = file.id();
            let req = QueueJobRequest {
                file_id,
                job_type: JobType::FileProcessing,
            };
            match self.queue_job_use_case.execute(tenant_id, req).await {
                Ok(_) => requeued += 1,
                Err(e) => eprintln!(
                    "[janitor] requeue failed for file {} (tenant={}): {}",
                    file_id, tenant_id, e
                ),
            }
        }
        requeued
    }
}

// `Uuid` is used in the trait signatures above; the explicit re-export
// here keeps that symbol reachable from the module root.
