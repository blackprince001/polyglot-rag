use std::fs;
use std::path::Path;

use async_trait::async_trait;

use super::ooxml_media::extract_media_assets;
use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions, PageContent,
};
use crate::domain::entities::File;

const DOCX_MIME: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

pub struct DocxExtractor;

impl DocxExtractor {
    pub fn new() -> Self {
        Self
    }

    fn can_extract_str(file_type: &str) -> bool {
        file_type.to_lowercase() == DOCX_MIME
    }

    fn text_from_docx(buf: &[u8]) -> Result<String, DocumentExtractionError> {
        let docx = docx_rs::read_docx(buf).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("invalid docx: {}", e))
        })?;
        Ok(collect_document_children(&docx.document.children))
    }

    fn document_from_bytes(buf: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
        let text = Self::text_from_docx(buf)?;
        let pending_assets = extract_media_assets(buf, "word/media/")?;
        Ok(ExtractedDocument {
            pages: vec![PageContent {
                page_number: 0,
                text: text.clone(),
                pending_assets,
            }],
            full_text: text,
            pending_assets: Vec::new(),
        })
    }

    fn document_from_path(path: &str) -> Result<ExtractedDocument, DocumentExtractionError> {
        if !Path::new(path).is_file() {
            return Err(DocumentExtractionError::CorruptedFile(format!(
                "docx file not found: {}",
                path
            )));
        }
        let bytes = fs::read(path).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("read failed: {}", e))
        })?;
        Self::document_from_bytes(&bytes)
    }
}

impl Default for DocxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentExtractor for DocxExtractor {
    async fn extract_text(
        &self,
        file: &File,
        _options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        Self::document_from_path(file.file_path())
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
        Self::document_from_bytes(data)
    }

    fn can_extract(&self, file_type: &str) -> bool {
        Self::can_extract_str(file_type)
    }
}

fn collect_document_children(children: &[docx_rs::DocumentChild]) -> String {
    let mut out = String::new();
    for child in children {
        match child {
            docx_rs::DocumentChild::Paragraph(p) => {
                out.push_str(&paragraph_text(p));
                out.push('\n');
            }
            docx_rs::DocumentChild::Table(t) => {
                out.push_str(&table_text(t));
                out.push('\n');
            }
            _ => {}
        }
    }
    out
}

fn paragraph_text(p: &docx_rs::Paragraph) -> String {
    let mut s = String::new();
    for child in &p.children {
        match child {
            docx_rs::ParagraphChild::Run(r) => {
                for run_child in &r.children {
                    if let docx_rs::RunChild::Text(t) = run_child {
                        s.push_str(&t.text);
                    }
                }
            }
            docx_rs::ParagraphChild::Hyperlink(h) => {
                for link_child in &h.children {
                    if let docx_rs::ParagraphChild::Run(r) = link_child {
                        for run_child in &r.children {
                            if let docx_rs::RunChild::Text(t) = run_child {
                                s.push_str(&t.text);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    s
}

fn table_text(t: &docx_rs::Table) -> String {
    let mut out = String::new();
    for row_child in &t.rows {
        let docx_rs::TableChild::TableRow(row) = row_child;
        for cell_child in &row.cells {
            let docx_rs::TableRowChild::TableCell(cell) = cell_child;
            for content in &cell.children {
                if let docx_rs::TableCellContent::Paragraph(p) = content {
                    out.push_str(&paragraph_text(p));
                    out.push('\t');
                }
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use docx_rs::{Docx, Paragraph, Run};

    fn build_minimal_docx(paragraphs: &[&str]) -> Vec<u8> {
        let mut doc = Docx::new();
        for p in paragraphs {
            doc = doc.add_paragraph(
                Paragraph::default().add_run(Run::default().add_text(p.to_string())),
            );
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        doc.build().pack(&mut buf).expect("failed to pack docx");
        buf.into_inner()
    }

    #[test]
    fn extracts_paragraphs_in_order() {
        let buf = build_minimal_docx(&["Hello", "World", "From docx-rs"]);
        let text = DocxExtractor::text_from_docx(&buf).expect("extract ok");
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(text.contains("From docx-rs"));
        let order_pos = text.find("Hello").unwrap();
        let world_pos = text.find("World").unwrap();
        let from_pos = text.find("From docx-rs").unwrap();
        assert!(order_pos < world_pos);
        assert!(world_pos < from_pos);
    }

    #[test]
    fn can_extract_matches_canonical_mime() {
        assert!(DocxExtractor::new().can_extract(DOCX_MIME));
        assert!(!DocxExtractor::new().can_extract("application/pdf"));
        assert!(!DocxExtractor::new().can_extract("text/plain"));
    }

    #[tokio::test]
    async fn rejects_wrong_mime() {
        let result = DocxExtractor::new()
            .extract_text_from_bytes(
                b"not a docx",
                "application/pdf",
                ExtractionOptions::default(),
            )
            .await;
        assert!(matches!(
            result,
            Err(DocumentExtractionError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn committed_fixture_emits_text_and_image_asset() {
        // tests/fixtures/sample.docx carries body text + word/media/image1.png
        // (generated by dev/smoke/make_fixtures.py).
        let bytes = std::fs::read("tests/fixtures/sample.docx").expect("read fixture");
        let doc = DocxExtractor::document_from_bytes(&bytes).expect("extract ok");
        assert!(doc.full_text.contains("Polyglot RAG"));
        let assets = doc.into_all_pending_assets();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].content_type, "image/png");
        assert_eq!(assets[0].page_number, Some(0));
        assert!(!assets[0].bytes.is_empty());
    }
}
