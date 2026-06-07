use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileHash(String);

impl FileHash {
    pub fn new(hash: String) -> Result<Self, String> {
        if hash.len() != 64 {
            return Err("Hash must be 64 characters long (SHA-256)".to_string());
        }

        if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Hash must contain only hexadecimal characters".to_string());
        }

        Ok(Self(hash.to_lowercase()))
    }

    pub fn from_bytes(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self(format!("{:x}", result))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, other: &FileHash) -> bool {
        self.0 == other.0
    }
}

impl std::fmt::Display for FileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<FileHash> for String {
    fn from(hash: FileHash) -> Self {
        hash.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hash() {
        let hash_str = "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let hash = FileHash::new(hash_str.to_string()).unwrap();
        assert_eq!(hash.as_str(), hash_str);
    }

    #[test]
    fn test_invalid_hash_length() {
        let hash_str = "invalid";
        let result = FileHash::new(hash_str.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_hash_characters() {
        let hash_str = "g665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3";
        let result = FileHash::new(hash_str.to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes() {
        let data = b"hello world";
        let hash = FileHash::from_bytes(data);
        assert_eq!(hash.as_str().len(), 64);
    }

    #[test]
    fn test_hash_matching() {
        let hash1 = FileHash::from_bytes(b"test data");
        let hash2 = FileHash::from_bytes(b"test data");
        let hash3 = FileHash::from_bytes(b"different data");

        assert!(hash1.matches(&hash2));
        assert!(!hash1.matches(&hash3));
    }
}
