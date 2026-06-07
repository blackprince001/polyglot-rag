use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use uuid::Uuid;

use crate::domain::entities::File;
use crate::domain::repositories::{FileRepository, file_repository::FileRepositoryError};
use crate::infrastructure::database::get_connection_from_pool;
use crate::infrastructure::database::models::{FileModel, NewFileModel};
use crate::infrastructure::database::schema::files::dsl::*;

pub struct PostgresFileRepository {
    pool: Pool<ConnectionManager<PgConnection>>,
}

impl PostgresFileRepository {
    pub fn new(pool: Pool<ConnectionManager<PgConnection>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FileRepository for PostgresFileRepository {
    async fn save(&self, tenant: Uuid, file: &File) -> Result<Uuid, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let new_file = NewFileModel::for_tenant(tenant, file);

        let inserted_file: FileModel = diesel::insert_into(files)
            .values(&new_file)
            .get_result(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted_file.id)
    }

    async fn find_by_id(
        &self,
        tenant: Uuid,
        id_val: Uuid,
    ) -> Result<Option<File>, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let result = files
            .filter(id.eq(id_val))
            .filter(tenant_id.eq(tenant))
            .first::<FileModel>(&mut conn)
            .optional()
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_file =
                    File::try_from(model).map_err(|e| FileRepositoryError::ValidationError(e))?;
                Ok(Some(domain_file))
            }
            None => Ok(None),
        }
    }

    async fn find_by_ids(
        &self,
        tenant: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<File>, FileRepositoryError> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let models = files
            .filter(id.eq_any(ids.to_vec()))
            .filter(tenant_id.eq(tenant))
            .load::<FileModel>(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let mut domain_files = Vec::new();
        for model in models {
            let domain_file =
                File::try_from(model).map_err(|e| FileRepositoryError::ValidationError(e))?;
            domain_files.push(domain_file);
        }

        Ok(domain_files)
    }

    async fn find_by_hash(
        &self,
        tenant: Uuid,
        hash: &str,
    ) -> Result<Option<File>, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let result = files
            .filter(file_hash.eq(hash))
            .filter(tenant_id.eq(tenant))
            .first::<FileModel>(&mut conn)
            .optional()
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        match result {
            Some(model) => {
                let domain_file =
                    File::try_from(model).map_err(|e| FileRepositoryError::ValidationError(e))?;
                Ok(Some(domain_file))
            }
            None => Ok(None),
        }
    }

    async fn find_all(
        &self,
        tenant: Uuid,
        skip: i64,
        limit: i64,
    ) -> Result<Vec<File>, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let models = files
            .filter(tenant_id.eq(tenant))
            .order(created_at.desc())
            .offset(skip)
            .limit(limit)
            .load::<FileModel>(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let mut domain_files = Vec::new();
        for model in models {
            let domain_file =
                File::try_from(model).map_err(|e| FileRepositoryError::ValidationError(e))?;
            domain_files.push(domain_file);
        }

        Ok(domain_files)
    }

    async fn update(&self, tenant: Uuid, file: &File) -> Result<(), FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let update_model = NewFileModel::for_tenant(tenant, file);

        diesel::update(files.filter(id.eq(file.id())).filter(tenant_id.eq(tenant)))
            .set(&update_model)
            .execute(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, tenant: Uuid, id_val: Uuid) -> Result<bool, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        let deleted_count =
            diesel::delete(files.filter(id.eq(id_val)).filter(tenant_id.eq(tenant)))
                .execute(&mut conn)
                .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted_count > 0)
    }

    async fn count(&self, tenant: Uuid) -> Result<i64, FileRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        files
            .filter(tenant_id.eq(tenant))
            .count()
            .get_result(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))
    }

    async fn find_stale_for_janitor(
        &self,
        threshold_secs: i64,
        statuses: &[crate::domain::value_objects::ProcessingStatus],
    ) -> Result<Vec<(File, Uuid)>, FileRepositoryError> {
        use chrono::{Duration, Utc};

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        // `ProcessingStatus` serializes with PascalCase variants via serde;
        // the DB column is text, so we mirror the same names.
        let status_strs: Vec<String> = statuses
            .iter()
            .map(|s| {
                serde_json::to_value(s)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default()
            })
            .collect();

        let threshold = Utc::now() - Duration::seconds(threshold_secs);

        let rows: Vec<(FileModel, Uuid)> = files
            .filter(processing_status.eq_any(&status_strs))
            .filter(updated_at.lt(threshold))
            .select((FileModel::as_select(), tenant_id))
            .load(&mut conn)
            .map_err(|e| FileRepositoryError::DatabaseError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(|(m, t)| (File::try_from(m).ok(), t))
            .filter_map(|(f, t)| f.map(|file| (file, t)))
            .collect())
    }
}
