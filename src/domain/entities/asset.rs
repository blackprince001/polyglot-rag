use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Image,
    Other,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::Image => "image",
            AssetType::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "image" => AssetType::Image,
            _ => AssetType::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    id: Uuid,
    tenant_id: Uuid,
    file_id: Uuid,
    asset_type: AssetType,
    storage_key: String,
    content_type: String,
    page_number: Option<i32>,
    label: Option<String>,
    byte_size: i64,
    created_at: DateTime<Utc>,
}

impl Asset {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: Uuid,
        file_id: Uuid,
        asset_type: AssetType,
        storage_key: String,
        content_type: String,
        page_number: Option<i32>,
        label: Option<String>,
        byte_size: i64,
    ) -> Self {
        Self::with_id(
            Uuid::new_v4(),
            tenant_id,
            file_id,
            asset_type,
            storage_key,
            content_type,
            page_number,
            label,
            byte_size,
            Utc::now(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_id(
        id: Uuid,
        tenant_id: Uuid,
        file_id: Uuid,
        asset_type: AssetType,
        storage_key: String,
        content_type: String,
        page_number: Option<i32>,
        label: Option<String>,
        byte_size: i64,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            file_id,
            asset_type,
            storage_key,
            content_type,
            page_number,
            label,
            byte_size,
            created_at,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn file_id(&self) -> Uuid {
        self.file_id
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn storage_key(&self) -> &str {
        &self.storage_key
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub fn page_number(&self) -> Option<i32> {
        self.page_number
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn byte_size(&self) -> i64 {
        self.byte_size
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
