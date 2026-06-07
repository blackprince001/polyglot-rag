use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::domain::entities::File as DomainFile;
use crate::domain::value_objects::{FileHash, FileMetadata, ProcessingStatus};
use crate::infrastructure::database::schema::files;

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Identifiable)]
#[diesel(table_name = files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct FileModel {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub file_path: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub file_type: Option<String>,
    pub file_hash: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub processing_status: String,
}

#[derive(Debug, Insertable, AsChangeset, Deserialize)]
#[diesel(table_name = files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewFileModel {
    pub id: Option<Uuid>,
    pub tenant_id: Uuid,
    pub file_path: String,
    pub file_name: String,
    pub file_size: Option<i64>,
    pub file_type: Option<String>,
    pub file_hash: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
    pub processing_status: String,
}

impl NewFileModel {
    pub fn for_tenant(tenant_id: Uuid, domain_file: &DomainFile) -> Self {
        Self {
            id: None, // Let database generate the ID
            tenant_id,
            file_path: domain_file.file_path().to_string(),
            file_name: domain_file.file_name().to_string(),
            file_size: domain_file.file_size(),
            file_type: domain_file.file_type().map(|s| s.to_string()),
            file_hash: domain_file.file_hash().map(|h| h.as_str().to_string()),
            created_at: Some(domain_file.created_at()),
            updated_at: Some(domain_file.updated_at()),
            metadata: domain_file.metadata().map(|m| m.clone().into()),
            processing_status: domain_file.processing_status().to_string(),
        }
    }
}

impl TryFrom<FileModel> for DomainFile {
    type Error = String;

    fn try_from(model: FileModel) -> Result<Self, Self::Error> {
        let file_hash = if let Some(hash_str) = model.file_hash {
            Some(FileHash::new(hash_str).map_err(|e| format!("Invalid file hash: {}", e))?)
        } else {
            None
        };

        let metadata = if let Some(metadata_json) = model.metadata {
            Some(
                FileMetadata::try_from(metadata_json)
                    .map_err(|e| format!("Invalid metadata: {}", e))?,
            )
        } else {
            None
        };

        let processing_status =
            ProcessingStatus::from_string(&model.processing_status).map_err(|e| {
                format!(
                    "Invalid processing status '{}': {}",
                    model.processing_status, e
                )
            })?;

        let domain_file = DomainFile::with_id(
            model.id,
            model.file_path,
            model.file_name,
            model.file_size,
            model.file_type,
            file_hash,
            model.created_at.unwrap_or_else(Utc::now),
            model.updated_at.unwrap_or_else(Utc::now),
            metadata,
            processing_status,
        );

        Ok(domain_file)
    }
}

impl TryFrom<serde_json::Value> for FileMetadata {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(map) => {
                let properties: HashMap<String, serde_json::Value> = map.into_iter().collect();
                Ok(FileMetadata::from(properties))
            }
            _ => Err("Metadata must be a JSON object".to_string()),
        }
    }
}
