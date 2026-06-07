use async_trait::async_trait;
use diesel::prelude::*;
use uuid::Uuid;

use crate::domain::entities::Asset;
use crate::domain::repositories::{AssetRepository, asset_repository::AssetRepositoryError};
use crate::infrastructure::database::models::{AssetModel, NewAssetModel};
use crate::infrastructure::database::schema::file_assets::dsl::*;
use crate::infrastructure::database::{DbPool, get_connection_from_pool};

pub struct PostgresAssetRepository {
    pool: DbPool,
}

impl PostgresAssetRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssetRepository for PostgresAssetRepository {
    async fn save_batch(
        &self,
        tenant: Uuid,
        assets: &[Asset],
    ) -> Result<Vec<Uuid>, AssetRepositoryError> {
        if assets.is_empty() {
            return Ok(Vec::new());
        }

        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        let new_rows: Vec<NewAssetModel> = assets
            .iter()
            .map(|a| NewAssetModel::for_tenant(tenant, a))
            .collect();

        let inserted: Vec<AssetModel> = diesel::insert_into(file_assets)
            .values(&new_rows)
            .get_results(&mut conn)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        Ok(inserted.into_iter().map(|a| a.id).collect())
    }

    async fn find_by_id(
        &self,
        tenant: Uuid,
        asset_id: Uuid,
    ) -> Result<Option<Asset>, AssetRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        let result = file_assets
            .filter(id.eq(asset_id))
            .filter(tenant_id.eq(tenant))
            .first::<AssetModel>(&mut conn)
            .optional()
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        Ok(result.map(Asset::from))
    }

    async fn find_by_file_id(
        &self,
        tenant: Uuid,
        file: Uuid,
    ) -> Result<Vec<Asset>, AssetRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        let models = file_assets
            .filter(file_id.eq(file))
            .filter(tenant_id.eq(tenant))
            .order(created_at.asc())
            .load::<AssetModel>(&mut conn)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        Ok(models.into_iter().map(Asset::from).collect())
    }

    async fn delete_by_file_id(
        &self,
        tenant: Uuid,
        file: Uuid,
    ) -> Result<i64, AssetRepositoryError> {
        let mut conn = get_connection_from_pool(&self.pool)
            .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        let deleted = diesel::delete(
            file_assets
                .filter(file_id.eq(file))
                .filter(tenant_id.eq(tenant)),
        )
        .execute(&mut conn)
        .map_err(|e| AssetRepositoryError::DatabaseError(e.to_string()))?;

        Ok(deleted as i64)
    }
}
