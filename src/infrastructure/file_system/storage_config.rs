use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use crate::application::ports::FileStorage;
use crate::infrastructure::file_system::{
    CloudinaryConfig, CloudinaryFileStorage, LocalFileStorage, S3Config, S3FileStorage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackend {
    Local,
    S3,
    Cloudinary,
}

impl StorageBackend {
    pub fn from_env() -> Result<Self, String> {
        let raw = env::var("STORAGE_BACKEND").unwrap_or_else(|_| "local".to_string());
        match raw.trim().to_ascii_lowercase().as_str() {
            "local" => Ok(StorageBackend::Local),
            "s3" => Ok(StorageBackend::S3),
            "cloudinary" => Ok(StorageBackend::Cloudinary),
            other => Err(format!(
                "Unknown STORAGE_BACKEND='{}'. Expected one of: local, s3, cloudinary",
                other
            )),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend: StorageBackend,
    pub upload_dir: PathBuf,
    pub s3: Option<S3Config>,
    pub cloudinary: Option<CloudinaryConfig>,
    pub presigned_upload_ttl_secs: u64,
    pub presigned_download_ttl_secs: u64,
}

impl StorageConfig {
    pub fn from_env() -> Result<Self, String> {
        let backend = StorageBackend::from_env()?;
        let upload_dir =
            PathBuf::from(env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string()));
        let s3 = match backend {
            StorageBackend::S3 => Some(S3Config::from_env()?),
            _ => None,
        };
        let cloudinary = match backend {
            StorageBackend::Cloudinary => Some(CloudinaryConfig::from_env()?),
            _ => None,
        };
        let presigned_upload_ttl_secs = env_u64("PRESIGNED_UPLOAD_TTL_SECS", 900);
        let presigned_download_ttl_secs = env_u64("PRESIGNED_DOWNLOAD_TTL_SECS", 300);
        Ok(Self {
            backend,
            upload_dir,
            s3,
            cloudinary,
            presigned_upload_ttl_secs,
            presigned_download_ttl_secs,
        })
    }

    pub fn presigned_upload_ttl_secs(&self) -> u64 {
        self.presigned_upload_ttl_secs
    }

    pub fn presigned_download_ttl_secs(&self) -> u64 {
        self.presigned_download_ttl_secs
    }

    pub async fn build(&self) -> Result<Arc<dyn FileStorage>, String> {
        match self.backend {
            StorageBackend::Local => {
                let storage = LocalFileStorage::new(self.upload_dir.clone());
                storage.ensure_directory_exists().await.map_err(|e| {
                    format!(
                        "Failed to create upload directory '{}': {}",
                        self.upload_dir.display(),
                        e
                    )
                })?;
                Ok(Arc::new(storage))
            }
            StorageBackend::S3 => {
                let cfg = self.s3.clone().ok_or_else(|| {
                    "S3 backend selected but S3Config missing (StorageConfig bug)".to_string()
                })?;
                let storage = S3FileStorage::new(cfg)
                    .await
                    .map_err(|e| format!("Failed to init S3 client: {}", e))?;
                Ok(Arc::new(storage))
            }
            StorageBackend::Cloudinary => {
                let cfg = self.cloudinary.clone().ok_or_else(|| {
                    "Cloudinary backend selected but CloudinaryConfig missing (StorageConfig bug)"
                        .to_string()
                })?;
                let storage = CloudinaryFileStorage::new(cfg)
                    .map_err(|e| format!("Failed to init Cloudinary client: {}", e))?;
                Ok(Arc::new(storage))
            }
        }
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}
