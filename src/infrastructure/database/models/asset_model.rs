use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;
use uuid::Uuid;

use crate::domain::entities::{Asset as DomainAsset, AssetType};
use crate::infrastructure::database::schema::file_assets;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Identifiable, Associations)]
#[diesel(belongs_to(super::FileModel, foreign_key = file_id))]
#[diesel(table_name = file_assets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct AssetModel {
    pub id: Uuid,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub asset_type: String,
    pub storage_key: String,
    pub content_type: String,
    pub page_number: Option<i32>,
    pub label: Option<String>,
    pub byte_size: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = file_assets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewAssetModel {
    pub id: Option<Uuid>,
    pub file_id: Uuid,
    pub tenant_id: Uuid,
    pub asset_type: String,
    pub storage_key: String,
    pub content_type: String,
    pub page_number: Option<i32>,
    pub label: Option<String>,
    pub byte_size: i64,
    pub created_at: DateTime<Utc>,
}

impl NewAssetModel {
    pub fn for_tenant(tenant_id: Uuid, asset: &DomainAsset) -> Self {
        Self {
            id: Some(asset.id()),
            file_id: asset.file_id(),
            tenant_id,
            asset_type: asset.asset_type().as_str().to_string(),
            storage_key: asset.storage_key().to_string(),
            content_type: asset.content_type().to_string(),
            page_number: asset.page_number(),
            label: asset.label().map(|s| s.to_string()),
            byte_size: asset.byte_size(),
            created_at: asset.created_at(),
        }
    }
}

impl From<AssetModel> for DomainAsset {
    fn from(model: AssetModel) -> Self {
        DomainAsset::with_id(
            model.id,
            model.tenant_id,
            model.file_id,
            AssetType::from_str(&model.asset_type),
            model.storage_key,
            model.content_type,
            model.page_number,
            model.label,
            model.byte_size,
            model.created_at,
        )
    }
}
