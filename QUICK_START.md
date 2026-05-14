# 30-Second Quick Start

## Install

```toml
[dependencies]
backyard = "0.1"
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

## Code

```rust
use backyard::{Job, JobContext, WorkerBuilder, SqliteQueue, Result};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Job, Serialize, Deserialize)]
struct SendEmail { to: String }

#[async_trait]
impl Job for SendEmail {
    async fn execute(self, _: &JobContext) -> Result<()> {
        println!("📧 Sent to {}", self.to);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let queue = SqliteQueue::new(Default::default()).await?;

    // Enqueue
    let job = SendEmail { to: "user@example.com".into() };
    let req = backyard::queue::EnqueueRequest {
        job_type: SendEmail::NAME.to_string(),
        queue: "default".to_string(),
        payload: serde_json::to_vec(&job)?,
        max_retries: 3,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };
    queue.push(req).await?;

    // Process
    WorkerBuilder::new(queue).concurrency(10).run().await?;

    Ok(())
}
```

## Run

```bash
cargo run
```

Done! ✨

**Learn more:** [USAGE.md](USAGE.md)
