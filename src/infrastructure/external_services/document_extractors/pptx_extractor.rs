use std::fs;
use std::io::Read;
use std::path::Path;

use async_trait::async_trait;
use regex::Regex;

use super::ooxml_media::extract_media_assets;
use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions, PageContent,
};
use crate::domain::entities::File;

const PPTX_MIME: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation";

pub struct PptxExtractor;

impl PptxExtractor {
    pub fn new() -> Self {
        Self
    }

    fn can_extract_str(file_type: &str) -> bool {
        file_type.to_lowercase() == PPTX_MIME
    }

    /// Build per-slide pages (1-based `page_number`) plus the concatenated
    /// `full_text`, and attach any embedded media (`ppt/media/*`) as
    /// document-level pending assets.
    fn document_from_bytes(buf: &[u8]) -> Result<ExtractedDocument, DocumentExtractionError> {
        let cursor = std::io::Cursor::new(buf);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            DocumentExtractionError::CorruptedFile(format!("invalid pptx zip: {}", e))
        })?;

        let mut slide_names: Vec<String> = (0..archive.len())
            .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
            .filter(|n| {
                n.starts_with("ppt/slides/slide") && n.ends_with(".xml") && !n.contains("_rels")
            })
            .collect();
        slide_names.sort_by_key(|n| slide_index(n));

        let mut pages = Vec::new();
        for name in slide_names {
            let mut entry = archive.by_name(&name).map_err(|e| {
                DocumentExtractionError::ExtractionFailed(format!("reading {}: {}", name, e))
            })?;
            let mut xml = String::new();
            entry.read_to_string(&mut xml).map_err(|e| {
                DocumentExtractionError::ExtractionFailed(format!("reading {}: {}", name, e))
            })?;
            drop(entry);
            pages.push(PageContent {
                page_number: slide_index(&name),
                text: extract_a_t_text(&xml),
                pending_assets: Vec::new(),
            });
        }

        let full_text = pages
            .iter()
            .map(|p| p.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let pending_assets = extract_media_assets(buf, "ppt/media/")?;

        Ok(ExtractedDocument {
            full_text,
            pages,
            pending_assets,
        })
    }

    fn document_from_path(path: &str) -> Result<ExtractedDocument, DocumentExtractionError> {
        if !Path::new(path).is_file() {
            return Err(DocumentExtractionError::CorruptedFile(format!(
                "pptx file not found: {}",
                path
            )));
        }
        let bytes = fs::read(path).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("read failed: {}", e))
        })?;
        Self::document_from_bytes(&bytes)
    }
}

fn slide_index(name: &str) -> u32 {
    name.trim_start_matches("ppt/slides/slide")
        .trim_end_matches(".xml")
        .parse::<u32>()
        .unwrap_or(u32::MAX)
}

fn extract_a_t_text(xml: &str) -> String {
    // DrawingML text runs are <a:t>...</a:t>. Captures everything between the
    // tags; XML entities (e.g. &amp;) get unescaped by reading the raw bytes
    // (the XML reader leaves them as entities).
    let re = Regex::new(r"(?s)<a:t[^>]*>(.*?)</a:t>").expect("valid regex");
    let mut out = String::new();
    for cap in re.captures_iter(xml) {
        let raw = &cap[1];
        out.push_str(&unescape_xml_entities(raw));
    }
    out
}

fn unescape_xml_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

impl Default for PptxExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DocumentExtractor for PptxExtractor {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn build_minimal_pptx(slides: &[&[&str]]) -> Vec<u8> {
        let mut zip_buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut zip_buf));
            let opts = SimpleFileOptions::default();

            w.start_file("[Content_Types].xml", opts).unwrap();
            w.write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
</Types>"#,
            )
            .unwrap();

            for (i, lines) in slides.iter().enumerate() {
                let name = format!("ppt/slides/slide{}.xml", i + 1);
                w.start_file(&name, opts).unwrap();
                let body: String = lines
                    .iter()
                    .map(|l| format!(r#"<a:t>{}</a:t>"#, l))
                    .collect();
                write!(
                    w,
                    r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>{}</p:spTree></p:cSld>
</p:sld>"#,
                    body
                )
                .unwrap();
            }
        }
        zip_buf
    }

    #[test]
    fn extracts_slides_in_numerical_order() {
        let buf = build_minimal_pptx(&[
            &["first slide", "second line"],
            &["second slide"],
            &["third slide", "with detail"],
        ]);
        let text = PptxExtractor::document_from_bytes(&buf)
            .expect("extract ok")
            .full_text;
        assert!(text.contains("first slide"));
        assert!(text.contains("second slide"));
        assert!(text.contains("third slide"));

        let p1 = text.find("first slide").unwrap();
        let p2 = text.find("second slide").unwrap();
        let p3 = text.find("third slide").unwrap();
        assert!(p1 < p2);
        assert!(p2 < p3);
    }

    #[test]
    fn unescapes_xml_entities() {
        assert_eq!(unescape_xml_entities("a &amp; b"), "a & b");
        assert_eq!(unescape_xml_entities("&lt;tag&gt;"), "<tag>");
        assert_eq!(unescape_xml_entities("&quot;hi&quot;"), "\"hi\"");
    }

    #[test]
    fn slide_index_parser() {
        assert_eq!(slide_index("ppt/slides/slide1.xml"), 1);
        assert_eq!(slide_index("ppt/slides/slide12.xml"), 12);
        assert_eq!(slide_index("ppt/slides/slide.xml"), u32::MAX);
    }

    #[test]
    fn can_extract_matches_canonical_mime() {
        assert!(PptxExtractor::new().can_extract(PPTX_MIME));
        assert!(!PptxExtractor::new().can_extract("application/pdf"));
        assert!(!PptxExtractor::new().can_extract("text/plain"));
    }

    #[tokio::test]
    async fn rejects_non_pptx_bytes() {
        let result = PptxExtractor::new()
            .extract_text_from_bytes(b"not a zip", PPTX_MIME, ExtractionOptions::default())
            .await;
        assert!(matches!(
            result,
            Err(DocumentExtractionError::CorruptedFile(_))
        ));
    }

    #[test]
    fn committed_fixture_emits_slides_and_image_asset() {
        // tests/fixtures/sample.pptx carries two slides + ppt/media/image1.png
        // (generated by dev/smoke/make_fixtures.py).
        let bytes = std::fs::read("tests/fixtures/sample.pptx").expect("read fixture");
        let doc = PptxExtractor::document_from_bytes(&bytes).expect("extract ok");
        assert_eq!(doc.pages.len(), 2);
        assert!(doc.full_text.contains("Polyglot RAG"));
        assert_eq!(doc.pending_assets.len(), 1);
        assert_eq!(doc.pending_assets[0].content_type, "image/png");
    }
}
