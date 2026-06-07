use async_trait::async_trait;
use std::sync::Arc;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions,
};
use crate::domain::entities::File;

use super::{
    DocxExtractor, HtmlExtractor, PdfExtractor, PptxExtractor, TextExtractor, YoutubeExtractor,
};

/// Dispatch table for `DocumentExtractor`. The first registered extractor whose
/// `can_extract` returns true wins. New formats register themselves in
/// [`ExtractorRegistry::with_defaults`]; no composite code changes required.
pub struct ExtractorRegistry {
    extractors: Vec<Arc<dyn DocumentExtractor>>,
}

impl ExtractorRegistry {
    pub fn new(extractors: Vec<Arc<dyn DocumentExtractor>>) -> Self {
        Self { extractors }
    }

    /// Built-in extractors. Order matters only when two extractors claim the
    /// same MIME (shouldn't happen); keep the most specific one first.
    pub fn with_defaults() -> Result<Self, DocumentExtractionError> {
        let registry = Self::new(vec![
            Arc::new(HtmlExtractor::new()),
            Arc::new(PdfExtractor::new()),
            Arc::new(YoutubeExtractor::new()?),
            Arc::new(DocxExtractor::new()),
            Arc::new(PptxExtractor::new()),
            Arc::new(TextExtractor::new()),
        ]);
        Ok(registry)
    }

    fn get_extractor_for_type(&self, file_type: &str) -> Option<Arc<dyn DocumentExtractor>> {
        self.extractors
            .iter()
            .find(|e| e.can_extract(file_type))
            .cloned()
    }
}

impl Default for ExtractorRegistry {
    fn default() -> Self {
        Self::with_defaults().expect("failed to build default extractor registry")
    }
}

#[async_trait]
impl DocumentExtractor for ExtractorRegistry {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        let file_type = file.file_type().ok_or_else(|| {
            DocumentExtractionError::UnsupportedFormat("no file_type set".to_string())
        })?;

        let extractor = self
            .get_extractor_for_type(file_type)
            .ok_or_else(|| DocumentExtractionError::UnsupportedFormat(file_type.to_string()))?;

        extractor.extract_text(file, options).await
    }

    async fn extract_text_from_bytes(
        &self,
        data: &[u8],
        file_type: &str,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        let extractor = self
            .get_extractor_for_type(file_type)
            .ok_or_else(|| DocumentExtractionError::UnsupportedFormat(file_type.to_string()))?;

        extractor
            .extract_text_from_bytes(data, file_type, options)
            .await
    }

    fn can_extract(&self, file_type: &str) -> bool {
        self.extractors.iter().any(|e| e.can_extract(file_type))
    }
}
