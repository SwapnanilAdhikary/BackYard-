# Backyard

A Sidekiq-inspired async job queue for Rust. Reliable, ergonomic, and production-ready.

## Features

- **Multiple Backends**: SQLite (no external dependencies) and Redis
- **Type-Safe Jobs**: `#[derive(Job)]` macro eliminates boilerplate
- **Reliable Delivery**: Automatic retries with exponential backoff
- **Worker Pool**: Configurable concurrency with Tokio integration
- **Observability**: Built-in tracing support
- **Web UI** (optional feature): Inspect and manage jobs
- **Cron Support** (optional feature): Schedule recurring jobs

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
backyard = { version = "0.1", features = ["sqlite"] }
serde = { version = "1", features = ["derive"] }
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
```

Define a job:

```rust
use backyard::{Job, JobContext, Result};
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

Enqueue and process jobs:

```rust
use backyard::{WorkerBuilder, SqliteQueue, SqliteConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let queue = SqliteQueue::new(SqliteConfig::default()).await?;
    
    // Enqueue a job
    let req = backyard::queue::EnqueueRequest {
        job_type: SendEmail::NAME.to_string(),
        queue: "default".to_string(),
        payload: serde_json::to_vec(&SendEmail {
            to: "user@example.com".into(),
            subject: "Hello".into(),
        })?,
        max_retries: 3,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };
    queue.push(req).await?;

    // Process jobs
    WorkerBuilder::new(queue)
        .concurrency(10)
        .queues(["default", "critical"])
        .run()
        .await?;

    Ok(())
}
```

## Architecture

```
User App → Queue Trait → Backend (SQLite/Redis)
                ↓
         Worker Pool → Job Handler
```

### Core Abstractions

- **`Job` Trait**: Define what your background jobs do
- **`Queue` Trait**: Pluggable backends (SQLite, Redis, custom)
- **`WorkerBuilder`**: Configure and run the worker pool
- **`JobContext`**: Access to the queue from within job handlers

## Backends

### SQLite (Default)

Perfect for small to medium applications that don't want external dependencies.

```rust
let queue = SqliteQueue::new(SqliteConfig {
    database_url: "sqlite://backyard.db".to_string(),
    ..Default::default()
}).await?;
```

### Redis

High-throughput, distributed job processing.

```rust
let queue = RedisQueue::new(RedisConfig {
    redis_url: "redis://localhost:6379".to_string(),
    pool_size: 10,
}).await?;
```

## Features

### Retry with Exponential Backoff

Failed jobs are automatically retried with configurable backoff:

```
Retry 0: 10-12.5s
Retry 1: 20-25s
Retry 2: 40-50s
...
Max: 1 hour
```

### Scheduled Jobs

Enqueue jobs for later:

```rust
let req = EnqueueRequest {
    scheduled_at: chrono::Utc::now() + chrono::Duration::hours(1),
    ..Default::default()
};
```

### Web UI (feature: `ui`)

```rust
let app = axum::Router::new()
    .nest("/backyard", worker_builder.ui_router());
```

Provides JSON API for:
- `GET /backyard/queues` - Queue depths
- `GET /backyard/jobs?queue=X&status=Y` - List jobs
- `POST /backyard/jobs/:id/retry` - Retry dead jobs

### Cron Support (feature: `cron`)

```rust
WorkerBuilder::new(queue)
    .cron("0 * * * *", SendDailyReport::default())
    .run()
    .await?
```

## Project Structure

```
backyard/
├── backyard-core/        # Job trait, Queue trait, Worker pool
├── backyard-sqlite/      # SQLite backend
├── backyard-redis/       # Redis backend (WIP)
├── backyard-macros/      # #[derive(Job)] proc-macro
└── backyard/             # Facade + re-exports
```

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests (SQLite)
cargo test --test basic

# All features
cargo test --all-features
```

## Roadmap for v1.0

- [ ] Redis backend implementation (currently stubs)
- [ ] Dead letter queue inspection UI
- [ ] Job pagination in web API
- [ ] Graceful shutdown with in-flight job tracking
- [ ] Metrics/Prometheus integration
- [ ] Enhanced tracing with context propagation

## Contributing

Contributions welcome! Areas of focus:

1. **Redis Backend**: Complete the fred client integration
2. **Testing**: More integration tests, especially for concurrency
3. **Documentation**: API docs, more examples
4. **Performance**: Benchmarks and optimization

## License

MIT OR Apache-2.0

## Inspiration

- [Sidekiq](https://sidekiq.org/) - Ruby job queue
- [Apalis](https://apalis.rs/) - Rust async job queue
- [Tokio](https://tokio.rs/) - Async runtime

## Author

Built as a Rust reimagining of Sidekiq's design patterns, optimizing for:

- Zero-boilerplate job definition via `#[derive(Job)]`
- Backend agnostic architecture
- First-class async/await support
- Production-ready reliability
