use std::fs;
use std::path::Path;

use async_trait::async_trait;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions,
};
use crate::domain::entities::File;

/// Trivial extractor for `text/plain` files (and aliases). The text is the
/// file's bytes decoded as UTF-8; no transformation is applied — chunking
/// happens downstream in the `RTSplitter`.
pub struct TextExtractor;

impl TextExtractor {
    pub fn new() -> Self {
        Self
    }

    fn can_extract_str(file_type: &str) -> bool {
        matches!(
            file_type.to_lowercase().as_str(),
            "text/plain" | "text/txt" | "txt" | "plain"
        )
    }
}

impl Default for TextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentExtractor for TextExtractor {
    async fn extract_text(
        &self,
        file: &File,
        _options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        let path = file.file_path();

        // The "path" may actually be a URL or any other opaque string. We only
        // treat it as a file path when one exists on disk; for anything else
        // (e.g. synthetic text-blob use) the bytes path is the right entry.
        if !Path::new(path).is_file() {
            return Err(DocumentExtractionError::CorruptedFile(format!(
                "text file not found on disk: {}",
                path
            )));
        }

        let bytes = fs::read(path).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("read failed: {}", e))
        })?;

        let text = String::from_utf8(bytes).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("invalid UTF-8: {}", e))
        })?;

        Ok(ExtractedDocument::text_only(text))
    }

    async fn extract_text_from_bytes(
        &self,
        data: &[u8],
        file_type: &str,
        _options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        if !Self::can_extract_str(file_type) {
            return Err(DocumentExtractionError::UnsupportedFormat(
                file_type.to_string(),
            ));
        }

        let text = String::from_utf8(data.to_vec()).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("invalid UTF-8: {}", e))
        })?;

        Ok(ExtractedDocument::text_only(text))
    }

    fn can_extract(&self, file_type: &str) -> bool {
        Self::can_extract_str(file_type)
    }
}
