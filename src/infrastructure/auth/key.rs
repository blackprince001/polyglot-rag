use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct GeneratedKey {
    pub raw: String,
    pub hash: String,
    pub prefix: String,
}

pub fn generate_api_key() -> GeneratedKey {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());

    let body = hex_encode(&bytes);
    let raw = format!("pk_live_{}", body);
    let hash = hash_key(&raw);
    let prefix = raw.chars().take(16).collect();

    GeneratedKey { raw, hash, prefix }
}

pub fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}
