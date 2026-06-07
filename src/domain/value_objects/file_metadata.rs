use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileMetadata {
    properties: HashMap<String, serde_json::Value>,
}

impl FileMetadata {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }

    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
    }

    pub fn get_property(&self, key: &str) -> Option<&serde_json::Value> {
        self.properties.get(key)
    }

    pub fn remove_property(&mut self, key: &str) -> Option<serde_json::Value> {
        self.properties.remove(key)
    }

    pub fn has_property(&self, key: &str) -> bool {
        self.properties.contains_key(key)
    }

    pub fn properties(&self) -> &HashMap<String, serde_json::Value> {
        &self.properties
    }

    pub fn is_empty(&self) -> bool {
        self.properties.is_empty()
    }

    pub fn len(&self) -> usize {
        self.properties.len()
    }

    // Common metadata helpers
    pub fn set_author(&mut self, author: String) {
        self.set_property("author".to_string(), serde_json::Value::String(author));
    }

    pub fn get_author(&self) -> Option<String> {
        self.get_property("author")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn set_title(&mut self, title: String) {
        self.set_property("title".to_string(), serde_json::Value::String(title));
    }

    pub fn get_title(&self) -> Option<String> {
        self.get_property("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn set_page_count(&mut self, count: i32) {
        self.set_property(
            "page_count".to_string(),
            serde_json::Value::Number(count.into()),
        );
    }

    pub fn get_page_count(&self) -> Option<i32> {
        self.get_property("page_count")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32)
    }

    pub fn set_language(&mut self, language: String) {
        self.set_property("language".to_string(), serde_json::Value::String(language));
    }

    pub fn get_language(&self) -> Option<String> {
        self.get_property("language")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn merge(&mut self, other: FileMetadata) {
        for (key, value) in other.properties {
            self.properties.insert(key, value);
        }
    }
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, serde_json::Value>> for FileMetadata {
    fn from(properties: HashMap<String, serde_json::Value>) -> Self {
        Self { properties }
    }
}

impl From<FileMetadata> for HashMap<String, serde_json::Value> {
    fn from(metadata: FileMetadata) -> Self {
        metadata.properties
    }
}

impl From<FileMetadata> for serde_json::Value {
    fn from(metadata: FileMetadata) -> Self {
        serde_json::Value::Object(
            metadata
                .properties
                .into_iter()
                .map(|(k, v)| (k, v))
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let metadata = FileMetadata::new();
        assert!(metadata.is_empty());
        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn test_property_operations() {
        let mut metadata = FileMetadata::new();

        metadata.set_property(
            "key1".to_string(),
            serde_json::Value::String("value1".to_string()),
        );
        assert!(metadata.has_property("key1"));
        assert_eq!(metadata.len(), 1);

        let value = metadata.get_property("key1").unwrap();
        assert_eq!(value.as_str().unwrap(), "value1");

        let removed = metadata.remove_property("key1").unwrap();
        assert_eq!(removed.as_str().unwrap(), "value1");
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_common_metadata_helpers() {
        let mut metadata = FileMetadata::new();

        metadata.set_author("John Doe".to_string());
        metadata.set_title("Test Document".to_string());
        metadata.set_page_count(10);
        metadata.set_language("en".to_string());

        assert_eq!(metadata.get_author().unwrap(), "John Doe");
        assert_eq!(metadata.get_title().unwrap(), "Test Document");
        assert_eq!(metadata.get_page_count().unwrap(), 10);
        assert_eq!(metadata.get_language().unwrap(), "en");
    }

    #[test]
    fn test_builder_pattern() {
        let metadata = FileMetadata::new()
            .with_property(
                "key1".to_string(),
                serde_json::Value::String("value1".to_string()),
            )
            .with_property("key2".to_string(), serde_json::Value::Number(42.into()));

        assert_eq!(metadata.len(), 2);
        assert!(metadata.has_property("key1"));
        assert!(metadata.has_property("key2"));
    }

    #[test]
    fn test_merge() {
        let mut metadata1 = FileMetadata::new();
        metadata1.set_author("Author 1".to_string());

        let mut metadata2 = FileMetadata::new();
        metadata2.set_title("Title 2".to_string());

        metadata1.merge(metadata2);

        assert_eq!(metadata1.get_author().unwrap(), "Author 1");
        assert_eq!(metadata1.get_title().unwrap(), "Title 2");
        assert_eq!(metadata1.len(), 2);
    }
}
