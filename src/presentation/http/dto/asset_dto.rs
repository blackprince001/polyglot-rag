use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::entities::Asset;

#[derive(Debug, Serialize, ToSchema)]
pub struct AssetDto {
    pub id: Uuid,
    pub asset_type: String,
    pub content_type: String,
    pub page_number: Option<i32>,
    pub label: Option<String>,
    pub byte_size: i64,
    pub download_url: String,
}

impl AssetDto {
    pub fn from_asset(asset: &Asset) -> Self {
        Self {
            id: asset.id(),
            asset_type: asset.asset_type().as_str().to_string(),
            content_type: asset.content_type().to_string(),
            page_number: asset.page_number(),
            label: asset.label().map(|s| s.to_string()),
            byte_size: asset.byte_size(),
            download_url: format!("/files/{}/assets/{}/content", asset.file_id(), asset.id()),
        }
    }
}
