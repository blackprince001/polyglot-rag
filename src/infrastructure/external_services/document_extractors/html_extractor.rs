use crate::domain::entities::File;
use async_trait::async_trait;
use html2text::from_read;
use url::Url;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions,
};
use crate::domain::value_objects::FileMetadata;

pub struct HtmlExtractor;

impl HtmlExtractor {
    pub fn new() -> Self {
        Self
    }

    // async fn extract_from_url(
    //     &self,
    //     url: &str,
    //     padding: usize,
    // ) -> Result<String, DocumentExtractionError> {
    //     // Validate URL
    //     Url::parse(url).map_err(|e| {
    //         DocumentExtractionError::ExtractionFailed(format!("Invalid URL: {}", e))
    //     })?;

    //     // Fetch HTML content
    //     let response = reqwest::get(url).await.map_err(|e| {
    //         DocumentExtractionError::ExtractionFailed(format!("Failed to fetch URL: {}", e))
    //     })?;

    //     let html_content = response.text().await.map_err(|e| {
    //         DocumentExtractionError::ExtractionFailed(format!("Failed to read response: {}", e))
    //     })?;

    //     // Convert HTML to text
    //     let text = from_read(html_content.as_bytes(), padding).map_err(|e| {
    //         DocumentExtractionError::ExtractionFailed(format!(
    //             "Failed to convert HTML to text: {}",
    //             e
    //         ))
    //     })?;

    //     Ok(text)
    // }

    async fn extract_from_html_content(
        &self,
        url: &str,
        padding: usize,
    ) -> Result<String, DocumentExtractionError> {
        let url = Url::parse(url).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Invalid URL: {}", e))
        })?;

        let response = reqwest::get(url).await.map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Failed to fetch URL: {}", e))
        })?;

        let html_content = response.text().await.map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Failed to read response: {}", e))
        })?;

        let text = from_read(html_content.as_bytes(), padding).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!(
                "Failed to convert HTML to text: {}",
                e
            ))
        })?;

        Ok(text)
    }
}

#[async_trait]
impl DocumentExtractor for HtmlExtractor {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        let content_path = file.file_path();
        let padding = 80; // Default padding for text width
        let text = self
            .extract_from_html_content(&content_path, padding)
            .await?;

        let _metadata = FileMetadata::new();
        if options.extract_metadata {
            let _ = extract_title_from_html(&content_path);
        }

        Ok(ExtractedDocument::text_only(text))
    }

    async fn extract_text_from_bytes(
        &self,
        data: &[u8],
        file_type: &str,
        _options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        if file_type != "text/html" && file_type != "application/html" {
            return Err(DocumentExtractionError::UnsupportedFormat(
                file_type.to_string(),
            ));
        }

        let html_content = String::from_utf8(data.to_vec()).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Invalid UTF-8: {}", e))
        })?;

        let padding = 80;
        let text = self
            .extract_from_html_content(&html_content, padding)
            .await?;

        Ok(ExtractedDocument::text_only(text))
    }

    fn can_extract(&self, file_type: &str) -> bool {
        matches!(
            file_type.to_lowercase().as_str(),
            "text/html" | "application/html" | "text/htm"
        )
    }
}

fn extract_title_from_html(html: &str) -> Option<String> {
    let title_regex = regex::Regex::new(r"<title[^>]*>([^<]+)</title>").ok()?;
    title_regex
        .captures(html)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().trim().to_string())
}

// pub async fn extract_from_url(url: &str) -> Result<ExtractedContent, DocumentExtractionError> {
//     let extractor = HtmlExtractor::new();
//     let text = extractor.extract_from_url(url, 80).await?;

//     let mut metadata = FileMetadata::new();
//     metadata.set_property(
//         "source_url".to_string(),
//         serde_json::Value::String(url.to_string()),
//     );

//     Ok(ExtractedContent {
//         text,
//         metadata,
//         page_count: Some(1),
//         language: Some("html".to_string()),
//     })
// }
