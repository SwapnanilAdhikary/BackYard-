//! # Backyard Core
//!
//! Core traits and types for the Backyard async job queue.
//!
//! This crate provides the fundamental abstractions that make Backyard backend-agnostic:
//! - [`Job`] trait for defining background jobs
//! - [`Queue`] trait for implementing backends (SQLite, Redis, etc.)
//! - [`WorkerPool`] for processing jobs with Tokio
//!
//! ## Quick Start
//!
//! Define a job by implementing [`Job`]:
//!
//! ```ignore
//! use backyard_core::{Job, JobContext, Result};
//! use async_trait::async_trait;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct SendEmail {
//!     to: String,
//! }
//!
//! #[async_trait]
//! impl Job for SendEmail {
//!     const NAME: &'static str = "SendEmail";
//!
//!     async fn execute(self, ctx: &JobContext) -> Result<()> {
//!         println!("Sending email to {}", self.to);
//!         Ok(())
//!     }
//! }
//! ```
//!
//! Then use with a backend like `backyard-sqlite`:
//!
//! ```ignore
//! use backyard_sqlite::SqliteQueue;
//! use backyard_core::WorkerBuilder;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let queue = SqliteQueue::new(Default::default()).await?;
//!
//!     // Enqueue a job
//!     let job = SendEmail { to: "user@example.com".into() };
//!     let payload = serde_json::to_vec(&job)?;
//!     let req = backyard_core::queue::EnqueueRequest {
//!         job_type: SendEmail::NAME.to_string(),
//!         queue: "default".to_string(),
//!         payload,
//!         max_retries: 3,
//!         priority: 0,
//!         scheduled_at: chrono::Utc::now(),
//!     };
//!     queue.push(req).await?;
//!
//!     // Run the worker
//!     WorkerBuilder::new(queue)
//!         .concurrency(10)
//!         .run()
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod job;
pub mod queue;
pub mod options;
pub mod worker;
pub mod registry;
pub mod retry;

pub use error::{BackyardError, Result};
pub use job::{Job, JobContext, JobId, RawJob};
pub use queue::Queue;
pub use options::JobOptions;
pub use worker::{WorkerPool, WorkerConfig};
pub use registry::build_dispatch_table;

pub use inventory;
