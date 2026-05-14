# Using Backyard in Your Project

Complete guide to using Backyard as a dependency in your Rust application.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
backyard = { version = "0.1", features = ["sqlite"] }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
chrono = "0.4"
```

## Define Your First Job

```rust
use backyard::{Job, JobContext, Result};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SendEmail {
    to: String,
    subject: String,
}

#[async_trait]
impl Job for SendEmail {
    const NAME: &'static str = "SendEmail";

    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("Sending email to {}: {}", self.to, self.subject);
        // Call your email service here
        Ok(())
    }
}
```

Or with the `#[derive(Job)]` macro:

```rust
use backyard::Job;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Job, Serialize, Deserialize)]
struct SendEmail {
    to: String,
    subject: String,
}

#[async_trait]
impl Job for SendEmail {
    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("Sending email to {}: {}", self.to, self.subject);
        Ok(())
    }
}
```

## Set Up the Queue

### SQLite (Default)

```rust
use backyard::SqliteQueue;

#[tokio::main]
async fn main() -> Result<()> {
    let queue = SqliteQueue::new(backyard::SqliteConfig {
        database_url: "sqlite://backyard.db".to_string(),
        ..Default::default()
    }).await?;

    Ok(())
}
```

### Redis

Enable the `redis` feature:

```toml
backyard = { version = "0.1", features = ["redis"] }
```

Then use:

```rust
use backyard::RedisQueue;

let queue = RedisQueue::new(backyard::RedisConfig {
    redis_url: "redis://localhost:6379".to_string(),
    pool_size: 10,
}).await?;
```

## Enqueue a Job

```rust
use backyard::queue::EnqueueRequest;
use chrono::Utc;

let job = SendEmail {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
};

let req = EnqueueRequest {
    job_type: SendEmail::NAME.to_string(),
    queue: "default".to_string(),
    payload: serde_json::to_vec(&job)?,
    max_retries: 3,
    priority: 0,
    scheduled_at: Utc::now(),
};

let job_id = queue.push(req).await?;
println!("Job enqueued: {}", job_id);
```

### Enqueue with Options

```rust
use backyard::JobOptions;

// Schedule for later
let scheduled_time = Utc::now() + Duration::hours(1);

let req = EnqueueRequest {
    job_type: SendEmail::NAME.to_string(),
    queue: "emails".to_string(),
    payload: serde_json::to_vec(&job)?,
    max_retries: 5,
    priority: 1,  // Higher priority
    scheduled_at: scheduled_time,
};

queue.push(req).await?;
```

## Run the Worker

```rust
use backyard::WorkerBuilder;

#[tokio::main]
async fn main() -> Result<()> {
    let queue = SqliteQueue::new(Default::default()).await?;

    WorkerBuilder::new(queue)
        .concurrency(10)                    // Run 10 jobs in parallel
        .queues(["default", "emails"])      // Watch these queues
        .run()
        .await?;

    Ok(())
}
```

## Full Example: Web Service with Background Jobs

```rust
use backyard::{Job, JobContext, Result, WorkerBuilder, SqliteQueue};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

// Define your jobs
#[derive(Serialize, Deserialize)]
struct SendWelcomeEmail {
    user_id: String,
    email: String,
}

#[async_trait]
impl Job for SendWelcomeEmail {
    const NAME: &'static str = "SendWelcomeEmail";

    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("Sending welcome email to {}", self.email);
        // Call email service
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct GenerateReport {
    report_id: String,
}

#[async_trait]
impl Job for GenerateReport {
    const NAME: &'static str = "GenerateReport";

    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("Generating report: {}", self.report_id);
        // Generate report...
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup queue
    let queue = Arc::new(
        SqliteQueue::new(Default::default()).await?
    );

    // Spawn worker in background
    let worker_queue = queue.clone();
    tokio::spawn(async move {
        WorkerBuilder::new(*worker_queue)
            .concurrency(10)
            .queues(["default", "reports"])
            .run()
            .await
    });

    // Simulate web requests enqueuing jobs
    let job = SendWelcomeEmail {
        user_id: "user_123".to_string(),
        email: "alice@example.com".to_string(),
    };

    let req = backyard::queue::EnqueueRequest {
        job_type: SendWelcomeEmail::NAME.to_string(),
        queue: "default".to_string(),
        payload: serde_json::to_vec(&job)?,
        max_retries: 3,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };

    queue.push(req).await?;
    println!("Enqueued welcome email job");

    // Keep running
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    Ok(())
}
```

## Error Handling & Retries

By default, failed jobs are retried with exponential backoff:

```
Attempt 0: 10-12.5 seconds later
Attempt 1: 20-25 seconds later
Attempt 2: 40-50 seconds later
Attempt 3: 80-100 seconds later
...
Max: 1 hour between retries
```

### Custom Retries

Control per-job:

```rust
let req = EnqueueRequest {
    // ...
    max_retries: 10,  // Default: 3
    // ...
};
```

Or per-job-type:

```rust
#[async_trait]
impl Job for MyJob {
    fn options() -> backyard::JobOptions {
        backyard::JobOptions::default()
            .max_retries(10)
            .queue("priority")
    }

    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        // ...
        Ok(())
    }
}
```

### Dead Letter Queue

Jobs that fail after max retries are moved to the **dead letter queue**:

```rust
// Query dead jobs
let dead_jobs = queue.list("default", Some("dead"), 100, 0).await?;

for job in dead_jobs {
    println!("Dead job: {:?}", job.error);
}
```

## Enqueue from Within a Job

Jobs can enqueue other jobs:

```rust
#[async_trait]
impl Job for ProcessOrder {
    async fn execute(self, ctx: &JobContext) -> Result<()> {
        println!("Processing order: {}", self.order_id);

        // Enqueue follow-up job
        let email = SendOrderConfirmation {
            order_id: self.order_id.clone(),
            email: "customer@example.com".to_string(),
        };

        ctx.enqueue(email, backyard::JobOptions::default()).await?;

        Ok(())
    }
}
```

## Testing Your Jobs

```rust
#[tokio::test]
async fn test_send_email() -> Result<()> {
    let job = SendEmail {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
    };

    let ctx = backyard::JobContext {
        queue: Arc::new(/* mock or real queue */),
        worker_id: "test".to_string(),
    };

    let result = job.execute(&ctx).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_enqueue_and_process() -> Result<()> {
    let queue = SqliteQueue::new(backyard::SqliteConfig {
        database_url: "sqlite://test.db".to_string(),
        ..Default::default()
    }).await?;

    // Enqueue
    let job = SendEmail { /* ... */ };
    let req = EnqueueRequest {
        job_type: SendEmail::NAME.to_string(),
        // ...
    };
    queue.push(req).await?;

    // Pop and verify
    let popped = queue.pop(&["default"]).await?;
    assert!(popped.is_some());

    Ok(())
}
```

## Integration with Web Frameworks

### Axum Example

```rust
use axum::{Router, routing::post, Json};
use serde_json::json;

async fn create_user(
    axum::extract::State(queue): axum::extract::State<Arc<SqliteQueue>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let email = body["email"].as_str().unwrap_or("unknown");

    let job = SendWelcomeEmail {
        user_id: uuid::Uuid::new_v4().to_string(),
        email: email.to_string(),
    };

    let req = backyard::queue::EnqueueRequest {
        job_type: SendWelcomeEmail::NAME.to_string(),
        queue: "default".to_string(),
        payload: serde_json::to_vec(&job).unwrap(),
        max_retries: 3,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };

    match queue.push(req).await {
        Ok(job_id) => Json(json!({ "job_id": job_id.to_string() })),
        Err(_) => Json(json!({ "error": "Failed to enqueue" })),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let queue = Arc::new(SqliteQueue::new(Default::default()).await?);

    let app = Router::new()
        .route("/users", post(create_user))
        .with_state(queue.clone());

    // Start web server...
    axum::Server::bind(&"127.0.0.1:3000".parse()?)
        .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
        .await?;

    Ok(())
}
```

## Monitoring & Debugging

### Check Queue Depths

```rust
let depths = queue.queue_depths().await?;
println!("Pending jobs: {:?}", depths);
```

### List All Jobs in a Queue

```rust
let jobs = queue.list("default", None, 100, 0).await?;
for job in jobs {
    println!("Job: type={}, attempts={}, scheduled={}", 
        job.job_type, job.attempts, job.scheduled_at);
}
```

### Get Specific Job Details

```rust
if let Some(job) = queue.get(job_id).await? {
    println!("Job details: {:#?}", job);
}
```

## Features

Enable additional features in `Cargo.toml`:

```toml
[dependencies]
backyard = { version = "0.1", features = ["sqlite", "redis", "ui", "cron"] }
```

- **sqlite**: SQLite backend (default)
- **redis**: Redis backend
- **ui**: JSON API for job inspection
- **cron**: Scheduled jobs via cron expressions

## Troubleshooting

### "Job type not registered"

Ensure your job type is in the same binary as the worker. The `#[derive(Job)]` macro uses `inventory` to auto-register jobs at compile time.

### "Database locked"

SQLite uses file-based locking. If you get lock timeouts:

```rust
let config = SqliteConfig {
    database_url: "sqlite://backyard.db".to_string(),
    poll_interval: Duration::from_millis(100),  // Faster polling
    max_connections: 5,  // Adjust as needed
};
```

### Jobs not executing

1. Verify worker is running: `WorkerBuilder::new(queue).run().await?`
2. Check queue name matches: Enqueued to `"default"`, worker watching `["default"]`
3. Verify job type name is correct: `SendEmail::NAME == "SendEmail"`

## Next Steps

- Check [TESTING.md](TESTING.md) for testing patterns
- Review [examples/](examples/) for complete code samples
- See [CLAUDE.md](CLAUDE.md) for architecture details
