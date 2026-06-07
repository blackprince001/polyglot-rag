use std::env;

#[derive(Debug, Clone)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub force_path_style: bool,
    pub access_key_id: String,
    pub secret_access_key: String,
}

impl S3Config {
    pub fn from_env() -> Result<Self, String> {
        let bucket = require_env("S3_BUCKET")?;
        let region = require_env("S3_REGION")?;
        let access_key_id = require_env("S3_ACCESS_KEY_ID")?;
        let secret_access_key = require_env("S3_SECRET_ACCESS_KEY")?;
        let endpoint = env::var("S3_ENDPOINT").ok().filter(|s| !s.is_empty());
        let force_path_style = env::var("S3_FORCE_PATH_STYLE")
            .ok()
            .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);
        Ok(Self {
            bucket,
            region,
            endpoint,
            force_path_style,
            access_key_id,
            secret_access_key,
        })
    }
}

fn require_env(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("STORAGE_BACKEND=s3 requires env var '{}'", name))
}
