use async_trait::async_trait;
use backyard_core::queue::EnqueueRequest;
use backyard_core::{BackyardError, Queue, RawJob, Result};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub struct RedisQueue {
    _config: crate::config::RedisConfig,
    worker_id: String,
}

impl RedisQueue {
    pub async fn new(config: crate::config::RedisConfig) -> Result<Self> {
        Ok(Self {
            _config: config,
            worker_id: Uuid::new_v4().to_string(),
        })
    }
}

#[async_trait]
impl Queue for RedisQueue {
    async fn push(&self, _req: EnqueueRequest) -> Result<Uuid> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn pop(&self, _queues: &[&str]) -> Result<Option<RawJob>> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn ack(&self, _id: Uuid) -> Result<()> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn fail(&self, _id: Uuid, _err: &str) -> Result<()> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn retry(&self, _id: Uuid, _retry_at: chrono::DateTime<Utc>) -> Result<()> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn get(&self, _id: Uuid) -> Result<Option<RawJob>> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn list(
        &self,
        _queue: &str,
        _status: Option<&str>,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<RawJob>> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }

    async fn queue_depths(&self) -> Result<HashMap<String, u64>> {
        Err(BackyardError::Backend(
            "Redis backend not fully implemented yet".to_string(),
        ))
    }
}
