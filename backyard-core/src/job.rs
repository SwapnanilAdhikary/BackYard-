use crate::error::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub type JobId = uuid::Uuid;

#[derive(Clone)]
pub struct JobContext {
    pub queue: Arc<dyn crate::queue::Queue>,
    pub worker_id: String,
}

impl JobContext {
    pub async fn enqueue<J: Job>(&self, job: J, opts: crate::options::JobOptions) -> Result<JobId> {
        let payload = serde_json::to_vec(&job)?;
        let scheduled_at = opts.scheduled_at();

        let req = crate::queue::EnqueueRequest {
            job_type: J::NAME.to_string(),
            queue: opts.queue,
            payload,
            max_retries: opts.max_retries,
            priority: opts.priority,
            scheduled_at,
        };

        self.queue.push(req).await
    }
}

/// A background job that can be enqueued and executed by workers.
///
/// # Implementing Job
///
/// Your job struct must:
/// 1. Implement `Serialize` and `Deserialize` (for storage)
/// 2. Implement `Send + Sync` (for Tokio task boundaries)
/// 3. Provide a `const NAME: &'static str` unique identifier
/// 4. Implement async `execute(self, ctx) -> Result<()>`
///
/// # Example
///
/// ```ignore
/// use backyard_core::{Job, JobContext, Result};
/// use async_trait::async_trait;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct SendEmail {
///     to: String,
///     subject: String,
/// }
///
/// #[async_trait]
/// impl Job for SendEmail {
///     const NAME: &'static str = "SendEmail";
///
///     async fn execute(self, _ctx: &JobContext) -> Result<()> {
///         println!("Sending {} to {}", self.subject, self.to);
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Job: serde::Serialize + serde::de::DeserializeOwned + Send + Sync + 'static {
    /// Unique identifier for this job type.
    /// Must be stable and unique across your entire application.
    const NAME: &'static str;

    /// Default enqueue options for this job type.
    /// Override to customize queue name, retries, priority, etc.
    fn options() -> crate::options::JobOptions {
        crate::options::JobOptions::default()
    }

    /// Execute the job.
    ///
    /// # Arguments
    /// * `ctx` - Job context including the queue (for enqueueing other jobs from within)
    ///
    /// # Returns
    /// * `Ok(())` - Job succeeded
    /// * `Err(BackyardError)` - Job failed; will be retried or moved to dead letter queue
    async fn execute(self, ctx: &JobContext) -> Result<()>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawJob {
    pub id: JobId,
    pub job_type: String,
    pub queue: String,
    pub payload: Vec<u8>,
    pub attempts: u32,
    pub max_retries: u32,
    pub scheduled_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}
