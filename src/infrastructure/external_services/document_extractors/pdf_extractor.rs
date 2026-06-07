use crate::domain::entities::File;
use async_trait::async_trait;
use lopdf::Document;
use lopdf::Object;
use lopdf::xobject::PdfImage;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::BTreeMap;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions, PageContent,
    PendingAsset,
};

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

    /// Load the PDF unfiltered and pull embedded image XObjects, grouped by the
    /// page they appear on. Best-effort: a malformed PDF or an undecodable image
    /// is skipped rather than failing the whole extraction.
    fn extract_images_by_page(path: &str) -> BTreeMap<u32, Vec<PendingAsset>> {
        let mut by_page: BTreeMap<u32, Vec<PendingAsset>> = BTreeMap::new();

        let doc = match Document::load(path) {
            Ok(d) => d,
            Err(_) => return by_page,
        };

        for (page_num, page_id) in doc.get_pages() {
            let images = match doc.get_page_images(page_id) {
                Ok(imgs) => imgs,
                Err(_) => continue,
            };
            for (idx, image) in images.iter().enumerate() {
                if let Some(asset) = pdf_image_to_asset(&doc, image, page_num, idx) {
                    by_page.entry(page_num).or_default().push(asset);
                }
            }
        }

        by_page
    }
}

impl Default for PdfExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert one extracted PDF image into a `PendingAsset`. Already-encoded image
/// streams (DCTDecode → JPEG, JPXDecode → JPEG2000) are passed through verbatim;
/// raw sample streams (FlateDecode / LZW for DeviceRGB or DeviceGray at 8 bits)
/// are decompressed and re-encoded as PNG. Other colorspaces are skipped.
fn pdf_image_to_asset(
    doc: &Document,
    image: &PdfImage,
    page_num: u32,
    idx: usize,
) -> Option<PendingAsset> {
    let filters = image.filters.clone().unwrap_or_default();
    let has = |name: &str| filters.iter().any(|f| f == name);

    let (bytes, ext, content_type) = if has("DCTDecode") {
        (image.content.to_vec(), "jpg", "image/jpeg")
    } else if has("JPXDecode") {
        (image.content.to_vec(), "jp2", "image/jp2")
    } else {
        // Raw samples — decompress via the stream's declared filters, then encode
        // a PNG for the colorspaces we understand.
        let samples = doc
            .get_object(image.id)
            .ok()?
            .as_stream()
            .ok()?
            .decompressed_content()
            .ok()?;
        let png = encode_png(
            &samples,
            image.width,
            image.height,
            image.color_space.as_deref(),
            image.bits_per_component,
        )?;
        (png, "png", "image/png")
    };

    if bytes.is_empty() {
        return None;
    }

    Some(PendingAsset {
        content_type: content_type.to_string(),
        bytes,
        page_number: Some(page_num),
        label: Some(format!("page{}-image{}.{}", page_num, idx + 1, ext)),
    })
}

/// Encode raw 8-bit DeviceRGB or DeviceGray samples as a PNG. Returns `None` for
/// unsupported bit depths or colorspaces (caller skips the image).
fn encode_png(
    samples: &[u8],
    width: i64,
    height: i64,
    color_space: Option<&str>,
    bits_per_component: Option<i64>,
) -> Option<Vec<u8>> {
    use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};

    if bits_per_component != Some(8) || width <= 0 || height <= 0 {
        return None;
    }
    let w = width as u32;
    let h = height as u32;
    let pixels = (w as usize).checked_mul(h as usize)?;

    let (color, needed) = match color_space {
        Some("DeviceRGB") | Some("RGB") | Some("CalRGB") => {
            (ColorType::Rgb8, pixels.checked_mul(3)?)
        }
        Some("DeviceGray") | Some("CalGray") | Some("G") => (ColorType::L8, pixels),
        _ => return None,
    };
    if samples.len() < needed {
        return None;
    }

    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(&samples[..needed], w, h, color.into())
        .ok()?;
    Some(out)
}

#[async_trait]
impl DocumentExtractor for PdfExtractor {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
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

        let (text, page_texts, _errors) = self.extract_pdf_text(&doc, &options).await?;

        // Embedded images are read from an unfiltered reload: `filter_func`
        // strips the Width/Height/Filter/ColorSpace keys we need to decode them.
        let mut images_by_page = Self::extract_images_by_page(file.file_path());

        let pages: Vec<PageContent> = page_texts
            .into_iter()
            .map(|(page_num, lines)| PageContent {
                page_number: page_num,
                text: lines.join("\n"),
                pending_assets: images_by_page.remove(&page_num).unwrap_or_default(),
            })
            .collect();

        // Any images on pages that produced no text rows still get surfaced at
        // the document level (their `page_number` is set on the asset).
        let leftover_assets: Vec<PendingAsset> = images_by_page.into_values().flatten().collect();

        Ok(ExtractedDocument {
            full_text: text,
            pages,
            pending_assets: leftover_assets,
        })
    }

    async fn extract_text_from_bytes(
        &self,
        _data: &[u8],
        _file_type: &str,
        _options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        unimplemented!()
    }

    fn can_extract(&self, file_type: &str) -> bool {
        file_type.to_lowercase() == "application/pdf"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG_MAGIC: &[u8] = &[0x89, b'P', b'N', b'G'];

    #[test]
    fn encodes_rgb8_samples_to_png() {
        // 2x2 RGB = 12 bytes.
        let samples = vec![0u8; 2 * 2 * 3];
        let png = encode_png(&samples, 2, 2, Some("DeviceRGB"), Some(8)).expect("rgb png");
        assert!(png.starts_with(PNG_MAGIC));
    }

    #[test]
    fn encodes_gray8_samples_to_png() {
        let samples = vec![128u8; 4 * 4];
        let png = encode_png(&samples, 4, 4, Some("DeviceGray"), Some(8)).expect("gray png");
        assert!(png.starts_with(PNG_MAGIC));
    }

    #[test]
    fn rejects_unsupported_depth_and_colorspace() {
        let samples = vec![0u8; 64];
        // 16-bit depth unsupported.
        assert!(encode_png(&samples, 2, 2, Some("DeviceRGB"), Some(16)).is_none());
        // CMYK colorspace unsupported.
        assert!(encode_png(&samples, 2, 2, Some("DeviceCMYK"), Some(8)).is_none());
    }

    #[test]
    fn rejects_truncated_sample_buffer() {
        // Claims 4x4 RGB (48 bytes) but only provides 10.
        let samples = vec![0u8; 10];
        assert!(encode_png(&samples, 4, 4, Some("DeviceRGB"), Some(8)).is_none());
    }
}
