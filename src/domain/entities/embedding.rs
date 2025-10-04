use chrono::{DateTime, Utc};
use pgvector::Vector;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    id: Uuid,
    content_chunk_id: Uuid,
    model_name: String,
    model_version: Option<String>,
    generated_at: DateTime<Utc>,
    generation_parameters: Option<serde_json::Value>,
    embedding: Vector,
}

impl Embedding {
    pub fn new(
        content_chunk_id: Uuid,
        model_name: String,
        model_version: Option<String>,
        generation_parameters: Option<serde_json::Value>,
        embedding: Vector,
    ) -> Self {
        Self {
            id: Uuid::nil(), // Will be set by database
            content_chunk_id,
            model_name,
            model_version,
            generated_at: Utc::now(),
            generation_parameters,
            embedding,
        }
    }

    pub fn with_id(
        id: Uuid,
        content_chunk_id: Uuid,
        model_name: String,
        model_version: Option<String>,
        generated_at: DateTime<Utc>,
        generation_parameters: Option<serde_json::Value>,
        embedding: Vector,
    ) -> Self {
        Self {
            id,
            content_chunk_id,
            model_name,
            model_version,
            generated_at,
            generation_parameters,
            embedding,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn content_chunk_id(&self) -> Uuid {
        self.content_chunk_id
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn model_version(&self) -> Option<&str> {
        self.model_version.as_deref()
    }

    pub fn generated_at(&self) -> DateTime<Utc> {
        self.generated_at
    }

    pub fn generation_parameters(&self) -> Option<&serde_json::Value> {
        self.generation_parameters.as_ref()
    }

    pub fn embedding(&self) -> &Vector {
        &self.embedding
    }

    pub fn dimension(&self) -> usize {
        self.embedding.as_slice().len()
    }

    pub fn is_compatible_with(&self, other: &Embedding) -> bool {
        self.model_name == other.model_name
            && self.model_version == other.model_version
            && self.dimension() == other.dimension()
    }

    pub fn cosine_similarity(&self, other: &Embedding) -> Result<f32, String> {
        if !self.is_compatible_with(other) {
            return Err("Embeddings are not compatible for similarity calculation".to_string());
        }

        let a = self.embedding.as_slice();
        let b = other.embedding.as_slice();

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Err("Cannot calculate similarity with zero vector".to_string());
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    pub fn magnitude(&self) -> f32 {
        self.embedding
            .as_slice()
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt()
    }

    pub fn belongs_to_chunk(&self, chunk_id: Uuid) -> bool {
        self.content_chunk_id == chunk_id
    }

    pub fn is_from_model(&self, model_name: &str, model_version: Option<&str>) -> bool {
        self.model_name == model_name && self.model_version.as_deref() == model_version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_vector(values: Vec<f32>) -> Vector {
        Vector::from(values)
    }

    #[test]
    fn test_embedding_creation() {
        let chunk_id = Uuid::new_v4();
        let vector = create_test_vector(vec![0.1, 0.2, 0.3]);

        let embedding = Embedding::new(
            chunk_id,
            "test-model".to_string(),
            Some("v1.0".to_string()),
            None,
            vector,
        );

        assert_eq!(embedding.content_chunk_id(), chunk_id);
        assert_eq!(embedding.model_name(), "test-model");
        assert_eq!(embedding.model_version(), Some("v1.0"));
        assert_eq!(embedding.dimension(), 3);
    }

    #[test]
    fn test_compatibility() {
        let chunk_id1 = Uuid::new_v4();
        let chunk_id2 = Uuid::new_v4();

        let embedding1 = Embedding::new(
            chunk_id1,
            "test-model".to_string(),
            Some("v1.0".to_string()),
            None,
            create_test_vector(vec![0.1, 0.2, 0.3]),
        );

        let embedding2 = Embedding::new(
            chunk_id2,
            "test-model".to_string(),
            Some("v1.0".to_string()),
            None,
            create_test_vector(vec![0.4, 0.5, 0.6]),
        );

        let embedding3 = Embedding::new(
            chunk_id2,
            "different-model".to_string(),
            Some("v1.0".to_string()),
            None,
            create_test_vector(vec![0.4, 0.5, 0.6]),
        );

        assert!(embedding1.is_compatible_with(&embedding2));
        assert!(!embedding1.is_compatible_with(&embedding3));
    }

    #[test]
    fn test_cosine_similarity() {
        let chunk_id1 = Uuid::new_v4();
        let chunk_id2 = Uuid::new_v4();

        let embedding1 = Embedding::new(
            chunk_id1,
            "test-model".to_string(),
            Some("v1.0".to_string()),
            None,
            create_test_vector(vec![1.0, 0.0, 0.0]),
        );

        let embedding2 = Embedding::new(
            chunk_id2,
            "test-model".to_string(),
            Some("v1.0".to_string()),
            None,
            create_test_vector(vec![1.0, 0.0, 0.0]),
        );

        let similarity = embedding1.cosine_similarity(&embedding2).unwrap();
        assert!((similarity - 1.0).abs() < 1e-6); // Should be 1.0 for identical vectors
    }
}
