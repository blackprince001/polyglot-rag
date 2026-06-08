use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::domain::entities::ProcessingJob;
use crate::domain::repositories::{JobRepository, job_repository::JobRepositoryError};
use crate::infrastructure::database::models::{JobModel, NewJobModel, UpdateJobModel};
use crate::infrastructure::database::schema::processing_jobs;

pub struct PostgresJobRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl PostgresJobRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }

    fn get_connection(
        &self,
    ) -> Result<diesel::r2d2::PooledConnection<ConnectionManager<PgConnection>>, JobRepositoryError>
    {
        self.pool.get().map_err(|e| {
            JobRepositoryError::DatabaseError(format!("Failed to get database connection: {}", e))
        })
    }
}

#[async_trait]
impl JobRepository for PostgresJobRepository {
    async fn save(&self, job: &ProcessingJob) -> Result<Uuid, JobRepositoryError> {
        let mut new_job = NewJobModel::from(job.clone());
        new_job.id = Some(job.id());
        let mut conn = self.get_connection()?;

        let inserted: JobModel = tokio::task::spawn_blocking(move || {
            diesel::insert_into(processing_jobs::table)
                .values(&new_job)
                .get_result::<JobModel>(&mut conn)
                .map_err(|e| {
                    JobRepositoryError::DatabaseError(format!("Failed to insert job: {}", e))
                })
        })
        .await
        .map_err(|e| JobRepositoryError::DatabaseError(format!("Task join error: {}", e)))??;

        Ok(inserted.id)
    }

    async fn find_by_id(
        &self,
        tenant_id: Uuid,
        job_id: Uuid,
    ) -> Result<Option<ProcessingJob>, JobRepositoryError> {
        let mut conn = self.get_connection()?;

        let result = tokio::task::spawn_blocking(move || {
            processing_jobs::table
                .filter(processing_jobs::id.eq(job_id))
                .filter(processing_jobs::tenant_id.eq(tenant_id))
                .first::<JobModel>(&mut conn)
                .optional()
                .map_err(|e| {
                    JobRepositoryError::DatabaseError(format!("Failed to find job: {}", e))
                })
        })
        .await
        .map_err(|e| JobRepositoryError::DatabaseError(format!("Task join error: {}", e)))??;

        match result {
            Some(job_model) => {
                let job = ProcessingJob::try_from(job_model).map_err(|e| {
                    JobRepositoryError::DatabaseError(format!("Failed to convert job model: {}", e))
                })?;
                Ok(Some(job))
            }
            None => Ok(None),
        }
    }

    async fn find_by_file_id(
        &self,
        tenant_id: Uuid,
        file_id: Uuid,
    ) -> Result<Vec<ProcessingJob>, JobRepositoryError> {
        let mut conn = self.get_connection()?;

        let job_models = tokio::task::spawn_blocking(move || {
            processing_jobs::table
                .filter(processing_jobs::file_id.eq(file_id))
                .filter(processing_jobs::tenant_id.eq(tenant_id))
                .order(processing_jobs::created_at.desc())
                .load::<JobModel>(&mut conn)
                .map_err(|e| {
                    JobRepositoryError::DatabaseError(format!(
                        "Failed to find jobs by file_id: {}",
                        e
                    ))
                })
        })
        .await
        .map_err(|e| JobRepositoryError::DatabaseError(format!("Task join error: {}", e)))??;

        let mut jobs = Vec::new();
        for job_model in job_models {
            let job = ProcessingJob::try_from(job_model).map_err(|e| {
                JobRepositoryError::DatabaseError(format!("Failed to convert job model: {}", e))
            })?;
            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn find_active_jobs(&self) -> Result<Vec<ProcessingJob>, JobRepositoryError> {
        let mut conn = self.get_connection()?;

        let job_models = tokio::task::spawn_blocking(move || {
            processing_jobs::table
                .filter(processing_jobs::status.eq_any(vec!["pending", "processing"]))
                .order(processing_jobs::created_at.asc())
                .load::<JobModel>(&mut conn)
                .map_err(|e| {
                    JobRepositoryError::DatabaseError(format!("Failed to find active jobs: {}", e))
                })
        })
        .await
        .map_err(|e| JobRepositoryError::DatabaseError(format!("Task join error: {}", e)))??;

        let mut jobs = Vec::new();
        for job_model in job_models {
            let job = ProcessingJob::try_from(job_model).map_err(|e| {
                JobRepositoryError::DatabaseError(format!("Failed to convert job model: {}", e))
            })?;
            jobs.push(job);
        }

        Ok(jobs)
    }

    async fn update(&self, job: &ProcessingJob) -> Result<(), JobRepositoryError> {
        let update_job = UpdateJobModel::from(job.clone());
        let job_id = job.id();
        let job_tenant = job.tenant_id();
        let mut conn = self.get_connection()?;

        let affected = tokio::task::spawn_blocking(move || {
            diesel::update(
                processing_jobs::table
                    .filter(processing_jobs::id.eq(job_id))
                    .filter(processing_jobs::tenant_id.eq(job_tenant)),
            )
            .set(&update_job)
            .execute(&mut conn)
            .map_err(|e| JobRepositoryError::DatabaseError(format!("Failed to update job: {}", e)))
        })
        .await
        .map_err(|e| JobRepositoryError::DatabaseError(format!("Task join error: {}", e)))??;

        if affected == 0 {
            tracing::warn!(
                job_id = %job_id,
                tenant_id = %job_tenant,
                "job update affected 0 rows; job row may be missing"
            );
        }

        Ok(())
    }
}
