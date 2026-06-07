use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

use crate::application::ports::job_queue::{JobQueue, JobQueueError};
use crate::domain::entities::processing_job::ProcessingJob;

pub struct MpscJobQueue {
    sender: mpsc::UnboundedSender<ProcessingJob>,
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<ProcessingJob>>>,
    pending_jobs: Arc<Mutex<HashMap<Uuid, ProcessingJob>>>, // For job removal
}

impl MpscJobQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();

        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            pending_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_pair() -> (Self, MpscJobQueueReceiver) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let pending_jobs = Arc::new(Mutex::new(HashMap::new()));

        let queue = Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            pending_jobs: pending_jobs.clone(),
        };

        // Create a separate receiver for the background processor
        let (bg_sender, bg_receiver) = mpsc::unbounded_channel();
        let bg_queue = MpscJobQueueReceiver {
            receiver: Arc::new(Mutex::new(bg_receiver)),
            sender: bg_sender,
        };

        // Forward messages from main queue to background queue
        let main_receiver = queue.receiver.clone();
        let bg_sender_clone = bg_queue.sender.clone();
        tokio::spawn(async move {
            loop {
                let job = {
                    let mut receiver = main_receiver.lock().await;
                    receiver.recv().await
                };

                match job {
                    Some(job) => {
                        if bg_sender_clone.send(job).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    None => break, // Channel closed
                }
            }
        });

        (queue, bg_queue)
    }
}

#[async_trait]
impl JobQueue for MpscJobQueue {
    async fn enqueue(&self, job: ProcessingJob) -> Result<(), JobQueueError> {
        // Track in pending map for cancellation support.
        {
            let mut pending = self.pending_jobs.lock().await;
            pending.insert(job.id(), job.clone());
        }

        // Send to channel.
        self.sender
            .send(job)
            .map_err(|_| JobQueueError::ConnectionError("Channel closed".to_string()))
    }

    async fn remove_job(&self, job_id: Uuid) -> Result<bool, JobQueueError> {
        let mut pending = self.pending_jobs.lock().await;
        Ok(pending.remove(&job_id).is_some())
    }
}

pub struct MpscJobQueueReceiver {
    receiver: Arc<Mutex<mpsc::UnboundedReceiver<ProcessingJob>>>,
    sender: mpsc::UnboundedSender<ProcessingJob>, // For forwarding
}

impl MpscJobQueueReceiver {
    pub async fn recv(&self) -> Option<ProcessingJob> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
}

impl Default for MpscJobQueue {
    fn default() -> Self {
        Self::new()
    }
}
