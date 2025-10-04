use crate::domain::entities::File;
use async_trait::async_trait;
use lopdf::Document;
use lopdf::Object;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::BTreeMap;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedContent, ExtractionOptions,
};
use crate::domain::value_objects::FileMetadata;

pub struct PdfExtractor {
    password: String,
}

impl PdfExtractor {
    pub fn new() -> Self {
        Self {
            password: String::new(),
        }
    }

    /// Sanitizes text extracted from PDF to ensure it's safe for database storage
    /// Removes null bytes and other problematic characters that can cause UTF-8 encoding issues
    fn sanitize_pdf_text(text: &str) -> String {
        text.chars()
            .filter(|c| *c != '\0') // Remove null bytes
            .filter(|c| {
                // Keep printable characters, whitespace, and common punctuation
                c.is_alphanumeric() || c.is_whitespace() || matches!(c, '!'..='~' | '¡'..='ÿ') // Printable ASCII + Latin-1
            })
            .collect::<String>()
    }

    // pub fn with_password(password: String) -> Self {
    //     Self { password }
    // }

    fn filter_func(object_id: (u32, u16), object: &mut Object) -> Option<((u32, u16), Object)> {
        static IGNORE: &[&[u8]] = &[
            b"Length",
            b"BBox",
            b"Matrix",
            b"Filter",
            b"ColorSpace",
            b"Width",
            b"Height",
            b"BitsPerComponent",
            b"PTEX.FileName",
            b"PTEX.PageNumber",
            b"PTEX.InfoDict",
            b"FontDescriptor",
            b"ExtGState",
            b"MediaBox",
        ];

        match object {
            Object::Dictionary(dict) => {
                let keys_to_remove: Vec<_> = dict
                    .iter()
                    .filter_map(|(key, _)| {
                        if IGNORE.contains(&key.as_slice()) {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for key in keys_to_remove {
                    dict.remove(&key);
                }
                // Don't filter out empty dictionaries - they might contain important structure
            }
            _ => {}
        }

        Some((object_id, object.to_owned()))
    }

    async fn extract_pdf_text(
        &self,
        doc: &Document,
        options: &ExtractionOptions,
    ) -> Result<(String, BTreeMap<u32, Vec<String>>, Vec<String>), DocumentExtractionError> {
        let pages = doc.get_pages();
        let mut errors = Vec::new();
        let mut page_texts = BTreeMap::new();

        // Filter pages if max_pages is specified
        let filtered_pages: BTreeMap<u32, (u32, u16)> = if let Some(max_pages) = options.max_pages {
            pages.into_iter().take(max_pages as usize).collect()
        } else {
            pages
        };

        let extracted_pages: Vec<Result<(u32, Vec<String>), String>> = filtered_pages
            .into_par_iter()
            .map(
                |(page_num, _): (u32, (u32, u16))| -> Result<(u32, Vec<String>), String> {
                    // Try extract_text method
                    let raw_text = doc.extract_text(&[page_num]).map_err(|e| {
                        format!("Failed to extract text from page {}: {}", page_num, e)
                    })?;

                    // Sanitize the text to remove null bytes and other invalid UTF-8 sequences
                    let sanitized_text = Self::sanitize_pdf_text(&raw_text);

                    // If sanitization resulted in empty text, provide a fallback message
                    let final_text = if sanitized_text.trim().is_empty() {
                        format!("[Page {}: No extractable text found - may contain images or corrupted text]", page_num)
                    } else {
                        sanitized_text
                    };

                    let lines: Vec<String> = final_text
                        .split('\n')
                        .map(|s| s.trim_end().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    Ok((page_num, lines))
                },
            )
            .collect();

        let mut all_text = Vec::new();

        for page_result in extracted_pages {
            match page_result {
                Ok((page_num, lines)) => {
                    page_texts.insert(page_num, lines.clone());
                    all_text.extend(lines);
                }
                Err(e) => {
                    errors.push(e);
                }
            }
        }

        let combined_text = all_text.join("\n");

        let final_text = if combined_text.trim().is_empty() {
            "No text could be extracted from this PDF. This might be an image-based PDF (scanned document) that requires OCR processing.".to_string()
        } else {
            combined_text
        };

        Ok((final_text, page_texts, errors))
    }

    fn extract_metadata_from_doc(&self, doc: &Document) -> FileMetadata {
        let mut metadata = FileMetadata::new();

        // Extract PDF metadata
        if let Ok(info) = doc.trailer.get(b"Info") {
            if let Ok(info_dict) = info.as_dict() {
                // Extract title
                if let Ok(title) = info_dict.get(b"Title") {
                    if let Ok(title_str) = title.as_str() {
                        if let Ok(title_utf8) = std::str::from_utf8(title_str) {
                            metadata.set_title(title_utf8.to_string());
                        } else {
                            metadata.set_title(String::from("[Invalid UTF-8 in Title]"));
                        }
                    }
                }

                // Extract author
                if let Ok(author) = info_dict.get(b"Author") {
                    if let Ok(author_str) = author.as_str() {
                        if let Ok(author_utf8) = std::str::from_utf8(author_str) {
                            metadata.set_author(author_utf8.to_string());
                        } else {
                            metadata.set_author(String::from("[Invalid UTF-8 in Author]"));
                        }
                    }
                }

                // Extract creation date, subject, etc.
                if let Ok(subject) = info_dict.get(b"Subject") {
                    if let Ok(subject_str) = subject.as_str() {
                        if let Ok(subject_utf8) = std::str::from_utf8(subject_str) {
                            metadata.set_author(subject_utf8.to_string());
                        } else {
                            metadata.set_author(String::from("[Invalid UTF-8 in Subject]"));
                        }
                    }
                }
            }

            metadata
        } else {
            FileMetadata::new()
        }
    }
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentExtractor for PdfExtractor {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedContent, DocumentExtractionError> {
        // let path = std::path::Path::new(&file.file_path());
        let mut doc = Document::load_filtered(file.file_path(), Self::filter_func)
            .map_err(|e| DocumentExtractionError::CorruptedFile(e.to_string()))?;

        if doc.is_encrypted() {
            doc.decrypt(&self.password).map_err(|_e| {
                DocumentExtractionError::ExtractionFailed(
                    "Failed to decrypt PDF - invalid password".to_string(),
                )
            })?;
        }

        let (text, page_texts, errors) = self.extract_pdf_text(&doc, &options).await?;

        let mut metadata = if options.extract_metadata {
            self.extract_metadata_from_doc(&doc)
        } else {
            FileMetadata::new()
        };

        let page_count = page_texts.len() as i32;
        metadata.set_page_count(page_count);
        metadata.set_language("pdf".to_string());

        if !errors.is_empty() {
            metadata.set_property(
                "extraction_errors".to_string(),
                serde_json::Value::Array(
                    errors.into_iter().map(serde_json::Value::String).collect(),
                ),
            );
        }

        Ok(ExtractedContent {
            text,
            metadata,
            page_count: Some(page_count),
            language: Some("pdf".to_string()),
        })
    }

    async fn extract_text_from_bytes(
        &self,
        _data: &[u8],
        _file_type: &str,
        _options: ExtractionOptions,
    ) -> Result<ExtractedContent, DocumentExtractionError> {
        unimplemented!()
    }

    fn supported_formats(&self) -> Vec<String> {
        vec!["application/pdf".to_string()]
    }

    fn can_extract(&self, file_type: &str) -> bool {
        file_type.to_lowercase() == "application/pdf"
    }

    fn max_file_size(&self) -> Option<usize> {
        Some(100 * 1024 * 1024) // 100MB max for PDF files
    }
}
