use crate::domain::entities::File;
use async_trait::async_trait;
use url::Url;
use yt_transcript_rs::api::YouTubeTranscriptApi;

use crate::application::ports::document_extractor::{
    DocumentExtractionError, DocumentExtractor, ExtractedDocument, ExtractionOptions,
};
use crate::domain::value_objects::FileMetadata;

pub struct YoutubeExtractor {
    api: YouTubeTranscriptApi,
}

impl YoutubeExtractor {
    pub fn new() -> Result<Self, DocumentExtractionError> {
        let api = YouTubeTranscriptApi::new(None, None, None).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Failed to setup YouTube API: {}", e))
        })?;

        Ok(Self { api })
    }

    pub async fn extract_from_url(
        &self,
        youtube_url: &str,
        options: &ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        // Parse URL and extract video ID
        let url = Url::parse(youtube_url).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Invalid YouTube URL: {}", e))
        })?;

        let video_id = self.extract_video_id(&url)?;

        // Fetch video details
        let details = self.api.fetch_video_details(&video_id).await.map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!(
                "Failed to fetch video details: {}",
                e
            ))
        })?;

        let languages = &["en"];

        let transcript = self
            .api
            .fetch_transcript(&video_id, languages, false)
            .await
            .map_err(|e| classify_transcript_error(&video_id, e))?;

        if transcript.snippets.is_empty() {
            return Err(DocumentExtractionError::NoTranscriptAvailable(format!(
                "video {} has no usable transcript captions",
                video_id
            )));
        }

        let mut timestamped_content = Vec::new();

        for snippet in &transcript.snippets {
            timestamped_content.push(format!(
                "[{:.1}-{:.1}s] {}",
                snippet.start,
                snippet.start + snippet.duration,
                snippet.text
            ));
        }

        // Create metadata
        let mut metadata = FileMetadata::new();
        if options.extract_metadata {
            metadata.set_title(details.title);
            metadata.set_author(details.author);
            metadata.set_property("video_id".to_string(), serde_json::Value::String(video_id));
            metadata.set_property(
                "channel_id".to_string(),
                serde_json::Value::String(details.channel_id),
            );
            metadata.set_property(
                "duration_seconds".to_string(),
                serde_json::Value::Number(details.length_seconds.into()),
            );
            metadata.set_property(
                "description".to_string(),
                serde_json::Value::String(details.short_description),
            );
            metadata.set_property(
                "source_url".to_string(),
                serde_json::Value::String(youtube_url.to_string()),
            );
            metadata.set_property(
                "timestamped_content".to_string(),
                serde_json::Value::Array(
                    timestamped_content
                        .clone()
                        .into_iter()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            );
        }

        let text = timestamped_content.clone().join("\n");
        let _ = metadata;

        Ok(ExtractedDocument::text_only(text))
    }

    fn extract_video_id(&self, url: &Url) -> Result<String, DocumentExtractionError> {
        // Handle different YouTube URL formats
        match url.host_str() {
            Some("www.youtube.com") | Some("youtube.com") => {
                // Standard format: https://www.youtube.com/watch?v=VIDEO_ID
                if let Some(_) = url.query() {
                    for (key, value) in url.query_pairs() {
                        if key == "v" {
                            return Ok(value.to_string());
                        }
                    }
                }
                Err(DocumentExtractionError::ExtractionFailed(
                    "Could not extract video ID from YouTube URL".to_string(),
                ))
            }
            Some("youtu.be") => {
                // Short format: https://youtu.be/VIDEO_ID
                if let Some(path) = url.path_segments() {
                    if let Some(video_id) = path.last() {
                        return Ok(video_id.to_string());
                    }
                }
                Err(DocumentExtractionError::ExtractionFailed(
                    "Could not extract video ID from short YouTube URL".to_string(),
                ))
            }
            _ => Err(DocumentExtractionError::ExtractionFailed(
                "Not a valid YouTube URL".to_string(),
            )),
        }
    }
}

impl Default for YoutubeExtractor {
    fn default() -> Self {
        Self::new().expect("Failed to create YouTube extractor")
    }
}

fn classify_transcript_error<E: std::fmt::Display>(
    video_id: &str,
    error: E,
) -> DocumentExtractionError {
    let raw = error.to_string();
    let lower = raw.to_lowercase();

    let no_transcript = lower.contains("no captions")
        || lower.contains("not parsable")
        || lower.contains("could not retrieve a transcript")
        || lower.contains("transcripts disabled")
        || lower.contains("transcripts are disabled")
        || lower.contains("subtitles are disabled")
        || lower.contains("no transcript")
        || lower.contains("transcript is unavailable");

    if no_transcript {
        DocumentExtractionError::NoTranscriptAvailable(format!(
            "video {}: captions are unavailable, disabled, or not provided by the source",
            video_id
        ))
    } else {
        DocumentExtractionError::ExtractionFailed(format!(
            "transcript fetch failed for video {}: {}",
            video_id, raw
        ))
    }
}

#[async_trait]
impl DocumentExtractor for YoutubeExtractor {
    async fn extract_text(
        &self,
        file: &File,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        let youtube_url = file.file_path();
        self.extract_from_url(youtube_url, &options).await
    }

    async fn extract_text_from_bytes(
        &self,
        data: &[u8],
        file_type: &str,
        options: ExtractionOptions,
    ) -> Result<ExtractedDocument, DocumentExtractionError> {
        if file_type != "text/youtube-url"
            && file_type != "text/youtube-transcript"
            && file_type != "text/plain"
        {
            return Err(DocumentExtractionError::UnsupportedFormat(
                file_type.to_string(),
            ));
        }

        let url_content = String::from_utf8(data.to_vec()).map_err(|e| {
            DocumentExtractionError::ExtractionFailed(format!("Invalid UTF-8: {}", e))
        })?;

        let youtube_url = url_content.trim();
        self.extract_from_url(youtube_url, &options).await
    }

    fn can_extract(&self, file_type: &str) -> bool {
        matches!(
            file_type.to_lowercase().as_str(),
            "text/youtube-url" | "text/youtube-transcript" | "application/youtube"
        )
    }
}

// pub async fn extract_youtube_transcript(
//     youtube_url: &str,
// ) -> Result<ExtractedContent, DocumentExtractionError> {
//     let extractor = YoutubeExtractor::new()?;
//     let options = ExtractionOptions {
//         extract_metadata: true,
//         max_pages: None,
//     };

//     extractor.extract_from_url(youtube_url, &options).await
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn innertube_no_captions_is_terminal() {
        let err = "Could not retrieve a transcript for the video 8ekndZwyOzo! \
                   This is most likely caused by: The data required to fetch the \
                   transcript is not parsable: No captions found in InnerTube response.";
        let classified = classify_transcript_error("8ekndZwyOzo", err);
        assert!(matches!(
            classified,
            DocumentExtractionError::NoTranscriptAvailable(_)
        ));
    }

    #[test]
    fn disabled_transcripts_is_terminal() {
        let classified = classify_transcript_error("abc", "Subtitles are disabled for this video");
        assert!(matches!(
            classified,
            DocumentExtractionError::NoTranscriptAvailable(_)
        ));
    }

    #[test]
    fn unknown_error_stays_generic_with_detail() {
        let classified = classify_transcript_error("abc", "connection reset by peer");
        match classified {
            DocumentExtractionError::ExtractionFailed(msg) => {
                assert!(msg.contains("connection reset by peer"));
                assert!(msg.contains("abc"));
            }
            other => panic!("expected ExtractionFailed, got {:?}", other),
        }
    }
}
