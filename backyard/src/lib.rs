//! # Backyard
//!
//! A Sidekiq-inspired async job queue for Rust.
//!
//! Backyard makes it easy to define, enqueue, and process background jobs with:
//! - **Type-safe jobs** via `#[derive(Job)]`
//! - **Reliable delivery** with automatic retries and dead letter queue
//! - **Multiple backends** (SQLite, Redis, or custom implementations)
//! - **Worker pool** with configurable concurrency
//! - **Observability** via tracing integration
//!
//! ## Quick Start
//!
//! ### 1. Define a Job
//!
//! ```ignore
//! use backyard::Job;
//! use async_trait::async_trait;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Job, Serialize, Deserialize)]
//! struct SendEmail {
//!     to: String,
//!     subject: String,
//! }
//!
//! #[async_trait]
//! impl Job for SendEmail {
//!     async fn execute(self, _ctx: &backyard::JobContext) -> backyard::Result<()> {
//!         println!("Sending email to {}: {}", self.to, self.subject);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ### 2. Enqueue the Job
//!
//! ```ignore
//! let queue = SqliteQueue::new(SqliteConfig::default()).await?;
//!
//! let job = SendEmail {
//!     to: "user@example.com".into(),
//!     subject: "Hello".into(),
//! };
//!
//! let req = backyard::queue::EnqueueRequest {
//!     job_type: SendEmail::NAME.to_string(),
//!     queue: "default".to_string(),
//!     payload: serde_json::to_vec(&job)?,
//!     max_retries: 3,
//!     priority: 0,
//!     scheduled_at: chrono::Utc::now(),
//! };
//!
//! queue.push(req).await?;
//! ```
//!
//! ### 3. Run the Worker
//!
//! ```ignore
//! WorkerBuilder::new(queue)
//!     .concurrency(10)
//!     .queues(["default"])
//!     .run()
//!     .await?;
//! ```
//!
//! ## Cargo.toml
//!
//! ```toml
//! [dependencies]
//! backyard = { version = "0.1", features = ["sqlite"] }
//! async-trait = "0.1"
//! serde = { version = "1", features = ["derive"] }
//! tokio = { version = "1", features = ["full"] }
//! chrono = "0.4"
//! ```
//!
//! ## Features
//!
//! - `sqlite` (default): SQLite backend, no external dependencies
//! - `redis`: Redis backend via `fred` client
//! - `ui`: JSON API for job inspection (requires Axum)
//! - `cron`: Scheduled job support via cron expressions
//!
//! ## Backends
//!
//! ### SQLite (Default)
//!
//! Perfect for single-server applications:
//!
//! ```ignore
//! use backyard::SqliteQueue;
//!
//! let queue = SqliteQueue::new(backyard::SqliteConfig {
//!     database_url: "sqlite://backyard.db".to_string(),
//!     ..Default::default()
//! }).await?;
//! ```
//!
//! ### Redis
//!
//! For distributed systems:
//!
//! ```ignore
//! use backyard::RedisQueue;
//!
//! let queue = RedisQueue::new(backyard::RedisConfig {
//!     redis_url: "redis://localhost:6379".to_string(),
//!     pool_size: 10,
//! }).await?;
//! ```
//!
//! ## Job Lifecycle
//!
//! 1. **Enqueue**: Job added to `pending` queue
//! 2. **Pop**: Worker picks up next available job
//! 3. **Execute**: Job's `async fn execute()` runs
//! 4. **Ack/Fail**: Success → deleted; Failure → retry or move to DLQ
//! 5. **Retry**: Failed jobs rescheduled with exponential backoff (10s→20s→40s...max 1h)
//!
//! ## Error Handling
//!
//! Jobs that fail are automatically retried according to `max_retries`:
//!
//! ```ignore
//! #[async_trait]
//! impl Job for MyJob {
//!     async fn execute(self, _ctx: &JobContext) -> Result<()> {
//!         // Returning Err() automatically triggers retry logic
//!         if something_went_wrong {
//!             return Err(backyard::BackyardError::Execution("reason".into()));
//!         }
//!         Ok(())
//!     }
//! }
//! ```
//!
//! After max retries exhausted, jobs are moved to the **dead letter queue**.

pub use backyard_core::{
    error::{BackyardError, Result},
    job::{Job, JobContext, JobId, RawJob},
    options::JobOptions,
    queue::Queue,
    worker::WorkerConfig,
};
pub use backyard_macros::Job;

pub mod builder;
pub use builder::WorkerBuilder;

/// Re-export queue types and request for convenience
pub mod queue {
    pub use backyard_core::queue::EnqueueRequest;
}

/// Retry logic for failed jobs
pub mod retry {
    pub use backyard_core::retry::next_retry_at;
}

#[cfg(feature = "redis")]
pub use backyard_redis::backend::RedisQueue;
#[cfg(feature = "redis")]
pub use backyard_redis::config::RedisConfig;

#[cfg(feature = "sqlite")]
pub use backyard_sqlite::backend::SqliteQueue;
#[cfg(feature = "sqlite")]
pub use backyard_sqlite::config::SqliteConfig;

#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "cron")]
pub mod cron;
