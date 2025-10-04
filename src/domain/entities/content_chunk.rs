use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentChunk {
    id: Uuid,
    file_id: Uuid,
    chunk_text: String,
    chunk_index: i32,
    token_count: Option<i32>,
    page_number: Option<i32>,
    section_path: Option<String>,
    created_at: DateTime<Utc>,
}

impl ContentChunk {
    pub fn new(
        file_id: Uuid,
        chunk_text: String,
        chunk_index: i32,
        token_count: Option<i32>,
        page_number: Option<i32>,
        section_path: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::nil(), // Will be set by database
            file_id,
            chunk_text,
            chunk_index,
            token_count,
            page_number,
            section_path,
            created_at: Utc::now(),
        }
    }

    pub fn with_id(
        id: Uuid,
        file_id: Uuid,
        chunk_text: String,
        chunk_index: i32,
        token_count: Option<i32>,
        page_number: Option<i32>,
        section_path: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            file_id,
            chunk_text,
            chunk_index,
            token_count,
            page_number,
            section_path,
            created_at,
        }
    }

    // Getters
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn file_id(&self) -> Uuid {
        self.file_id
    }

    pub fn chunk_text(&self) -> &str {
        &self.chunk_text
    }

    pub fn chunk_index(&self) -> i32 {
        self.chunk_index
    }

    pub fn token_count(&self) -> Option<i32> {
        self.token_count
    }

    pub fn page_number(&self) -> Option<i32> {
        self.page_number
    }

    pub fn section_path(&self) -> Option<&str> {
        self.section_path.as_deref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    // Business logic methods
    pub fn is_empty(&self) -> bool {
        self.chunk_text.trim().is_empty()
    }

    pub fn word_count(&self) -> usize {
        self.chunk_text.split_whitespace().count()
    }

    pub fn character_count(&self) -> usize {
        self.chunk_text.len()
    }

    pub fn update_token_count(&mut self, count: i32) {
        self.token_count = Some(count);
    }

    pub fn has_meaningful_content(&self) -> bool {
        !self.is_empty() && self.word_count() >= 3
    }

    pub fn belongs_to_file(&self, file_id: Uuid) -> bool {
        self.file_id == file_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_creation() {
        let file_id = Uuid::new_v4();
        let chunk = ContentChunk::new(
            file_id,
            "This is a test chunk with some content.".to_string(),
            0,
            Some(10),
            Some(1),
            Some("section1".to_string()),
        );

        assert_eq!(chunk.file_id(), file_id);
        assert_eq!(chunk.chunk_index(), 0);
        assert_eq!(chunk.token_count(), Some(10));
        assert!(!chunk.is_empty());
        assert!(chunk.has_meaningful_content());
    }

    #[test]
    fn test_empty_chunk() {
        let file_id = Uuid::new_v4();
        let chunk = ContentChunk::new(file_id, "   ".to_string(), 0, None, None, None);

        assert!(chunk.is_empty());
        assert!(!chunk.has_meaningful_content());
        assert_eq!(chunk.word_count(), 0);
    }

    #[test]
    fn test_word_and_character_count() {
        let file_id = Uuid::new_v4();
        let chunk = ContentChunk::new(file_id, "Hello world test".to_string(), 0, None, None, None);

        assert_eq!(chunk.word_count(), 3);
        assert_eq!(chunk.character_count(), 16);
    }
}
