use crate::domain::entities::File;
use async_trait::async_trait;

#[derive(Debug)]
pub enum DocumentExtractionError {
    UnsupportedFormat(String),
    CorruptedFile(String),
    ExtractionFailed(String),
    NoTranscriptAvailable(String),
}

impl std::fmt::Display for DocumentExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentExtractionError::UnsupportedFormat(format) => {
                write!(f, "Unsupported format: {}", format)
            }
            DocumentExtractionError::CorruptedFile(msg) => write!(f, "Corrupted file: {}", msg),
            DocumentExtractionError::ExtractionFailed(msg) => {
                write!(f, "Extraction failed: {}", msg)
            }
            DocumentExtractionError::NoTranscriptAvailable(msg) => {
                write!(f, "No transcript available: {}", msg)
            }
        }
    }
}

impl std::error::Error for DocumentExtractionError {}

#[derive(Debug, Clone)]
pub struct ExtractedDocument {
    pub full_text: String,
    pub pages: Vec<PageContent>,
    pub pending_assets: Vec<PendingAsset>,
}

#[derive(Debug, Clone)]
pub struct PageContent {
    pub page_number: u32,
    pub text: String,
    pub pending_assets: Vec<PendingAsset>,
}

#[derive(Debug, Clone)]
pub struct PendingAsset {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub page_number: Option<u32>,
    pub label: Option<String>,
}

impl ExtractedDocument {
    pub fn text_only(text: String) -> Self {
        Self {
            pages: vec![PageContent {
                page_number: 0,
                text: text.clone(),
                pending_assets: Vec::new(),
            }],
            full_text: text,
            pending_assets: Vec::new(),
        }
    }

    pub fn into_all_pending_assets(self) -> Vec<PendingAsset> {
        let mut out = self.pending_assets;
        for page in self.pages {
            for mut asset in page.pending_assets {
                if asset.page_number.is_none() {
                    asset.page_number = Some(page.page_number);
                }
                out.push(asset);
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    pub extract_metadata: bool,
    pub max_pages: Option<i32>,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            extract_metadata: true,
            max_pages: None,
        }
    }
}

#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError>;

    async fn extract_text_from_bytes(
        &self,
        data: &[u8],
        file_type: &str,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError>;

    fn can_extract(&self, file_type: &str) -> bool;
}
