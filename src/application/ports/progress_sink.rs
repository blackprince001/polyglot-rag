use async_trait::async_trait;

/// Receives progress from long-running document processing. The embedding
/// phase dominates a document's wall-clock, and without per-batch reporting a
/// job sits at one coarse figure for its entire run — which a client cannot
/// distinguish from a hang.
#[async_trait]
pub trait ProgressSink: Send + Sync {
    /// `fraction` is overall job completion in [0, 1].
    async fn report(&self, fraction: f32, message: Option<String>);
}
