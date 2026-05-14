use async_trait::async_trait;
use crate::error::Result;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

pub struct EnqueueRequest {
    pub job_type: String,
    pub queue: String,
    pub payload: Vec<u8>,
    pub max_retries: u32,
    pub priority: i32,
    pub scheduled_at: DateTime<Utc>,
}

pub use crate::job::{JobId, RawJob};

#[async_trait]
pub trait Queue: Send + Sync + 'static {
    async fn push(&self, req: EnqueueRequest) -> Result<JobId>;

    async fn pop(&self, queues: &[&str]) -> Result<Option<RawJob>>;

    async fn ack(&self, id: JobId) -> Result<()>;

    async fn fail(&self, id: JobId, err: &str) -> Result<()>;

    async fn retry(&self, id: JobId, retry_at: DateTime<Utc>) -> Result<()>;

    async fn get(&self, id: JobId) -> Result<Option<RawJob>>;

    async fn list(
        &self,
        queue: &str,
        status: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RawJob>>;

    async fn queue_depths(&self) -> Result<HashMap<String, u64>>;
}
