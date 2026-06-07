use std::env;

#[derive(Debug, Clone)]
pub struct CloudinaryConfig {
    pub cloud_name: String,
    pub api_key: String,
    pub api_secret: String,
    pub folder: Option<String>,
}

impl CloudinaryConfig {
    pub fn from_env() -> Result<Self, String> {
        let cloud_name = require_env("CLOUDINARY_CLOUD_NAME")?;
        let api_key = require_env("CLOUDINARY_API_KEY")?;
        let api_secret = require_env("CLOUDINARY_API_SECRET")?;
        let folder = env::var("CLOUDINARY_FOLDER").ok().filter(|s| !s.is_empty());
        Ok(Self {
            cloud_name,
            api_key,
            api_secret,
            folder,
        })
    }

    pub fn basic_auth_header(&self) -> String {
        use base64::Engine;
        let raw = format!("{}:{}", self.api_key, self.api_secret);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw)
        )
    }

    pub fn public_id(&self, tenant_id: uuid::Uuid, file_id: uuid::Uuid) -> String {
        match &self.folder {
            Some(folder) => format!("{}/{}/{}", folder, tenant_id, file_id),
            None => format!("{}/{}", tenant_id, file_id),
        }
    }
}

fn require_env(name: &str) -> Result<String, String> {
    env::var(name)
        .ok()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("STORAGE_BACKEND=cloudinary requires env var '{}'", name))
}
