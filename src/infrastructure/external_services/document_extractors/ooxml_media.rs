//! Shared helper for pulling embedded media out of OOXML packages (docx/pptx).
//!
//! Both formats are ZIP (OPC) packages that store binary assets under a media
//! folder — `word/media/` for docx, `ppt/media/` for pptx. We read every entry
//! under the given prefix and turn it into a [`PendingAsset`] tagged with a
//! best-effort content type derived from the file extension.

use std::io::Read;

use crate::application::ports::document_extractor::{DocumentExtractionError, PendingAsset};

pub fn extract_media_assets(
    buf: &[u8],
    prefix: &str,
) -> Result<Vec<PendingAsset>, DocumentExtractionError> {
    let cursor = std::io::Cursor::new(buf);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| DocumentExtractionError::CorruptedFile(format!("invalid OOXML zip: {}", e)))?;

    let mut names: Vec<String> = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .filter(|n| n.starts_with(prefix) && !n.ends_with('/'))
        .collect();
    // Deterministic order so re-runs label assets consistently.
    names.sort();

    let mut assets = Vec::new();
    for name in names {
        let mut entry = archive.by_name(&name).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("reading {}: {}", name, e))
        })?;
        let mut bytes = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut bytes).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("reading {}: {}", name, e))
        })?;
        if bytes.is_empty() {
            continue;
        }
        let label = name.rsplit('/').next().unwrap_or(&name).to_string();
        assets.push(PendingAsset {
            content_type: content_type_for(&label),
            bytes,
            page_number: None,
            label: Some(label),
        });
    }

    Ok(assets)
}

fn content_type_for(name: &str) -> String {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "emf" => "image/emf",
        "wmf" => "image/wmf",
        _ => "application/octet-stream",
    };
    mime.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            for (name, bytes) in entries {
                w.start_file(*name, opts).unwrap();
                w.write_all(bytes).unwrap();
            }
            w.finish().unwrap();
        }
        buf
    }

    #[test]
    fn content_type_by_extension() {
        assert_eq!(content_type_for("image1.png"), "image/png");
        assert_eq!(content_type_for("photo.JPEG"), "image/jpeg");
        assert_eq!(content_type_for("logo.svg"), "image/svg+xml");
        assert_eq!(content_type_for("mystery.dat"), "application/octet-stream");
    }

    #[test]
    fn extracts_only_media_under_prefix() {
        let buf = zip_with(&[
            ("word/document.xml", b"<w:document/>"),
            ("word/media/image1.png", b"\x89PNG\r\nfake"),
            ("word/media/image2.jpeg", b"\xff\xd8\xfffake"),
            ("word/media/empty.png", b""),
        ]);

        let assets = extract_media_assets(&buf, "word/media/").expect("ok");
        // Two non-empty media; the empty entry is skipped.
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0].label.as_deref(), Some("image1.png"));
        assert_eq!(assets[0].content_type, "image/png");
        assert_eq!(assets[1].content_type, "image/jpeg");
        assert!(assets.iter().all(|a| !a.bytes.is_empty()));
    }

    #[test]
    fn no_media_yields_empty() {
        let buf = zip_with(&[("ppt/slides/slide1.xml", b"<p:sld/>")]);
        let assets = extract_media_assets(&buf, "ppt/media/").expect("ok");
        assert!(assets.is_empty());
    }
}
