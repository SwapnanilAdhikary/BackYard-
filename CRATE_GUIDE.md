# Backyard Crate User Guide

## 📦 Overview

**Backyard** is production-ready to use as a Rust crate. This guide shows everything available to users.

## 🚀 Install from Crates.io (When Published)

```toml
[dependencies]
backyard = "0.1"
```

## 📚 Documentation

### In-Code Documentation

All public APIs have rustdoc comments. View locally:

```bash
cargo doc --open
```

This generates HTML docs with:
- Module-level documentation
- Trait documentation with examples
- Function/struct field descriptions
- Code examples you can copy-paste

### Files for Users

| File | Purpose |
|------|---------|
| [README.md](README.md) | Quick overview, architecture, features |
| [USAGE.md](USAGE.md) | **Complete user guide** with examples |
| [TESTING.md](TESTING.md) | How to test your jobs |
| [examples/](examples/) | Working code samples |

**Start here:** Read [USAGE.md](USAGE.md)

## 🔧 What's Available to Users

### Core Types

```rust
use backyard::{
    Job,              // Trait to implement
    JobContext,       // Passed to execute()
    WorkerBuilder,    // Configures and runs workers
    JobOptions,       // Enqueue configuration
    Result,           // backyard::Result<T>
    BackyardError,    // Error type
};
```

### Backends

```rust
// SQLite (default, included by default)
use backyard::SqliteQueue;

// Redis (optional, feature = "redis")
use backyard::RedisQueue;
```

### Features

```toml
# Minimal (SQLite only)
backyard = "0.1"

# With Redis
backyard = { version = "0.1", features = ["redis"] }

# Everything
backyard = { version = "0.1", features = ["redis", "ui", "cron"] }
```

## 💡 Quick Usage

### 1. Define a Job

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

### 2. Enqueue It

```rust
let req = backyard::queue::EnqueueRequest {
    job_type: SendEmail::NAME.to_string(),
    queue: "default".to_string(),
    payload: serde_json::to_vec(&job)?,
    max_retries: 3,
    priority: 0,
    scheduled_at: chrono::Utc::now(),
};

queue.push(req).await?;
```

### 3. Run the Worker

```rust
WorkerBuilder::new(queue)
    .concurrency(10)
    .run()
    .await?;
```

**Full example in [USAGE.md](USAGE.md)**

## 🧪 Testing

Backyard provides test infrastructure:

```rust
#[tokio::test]
async fn test_my_job() -> Result<()> {
    let queue = SqliteQueue::new(Default::default()).await?;
    
    // Enqueue, pop, execute...
    
    Ok(())
}
```

See [TESTING.md](TESTING.md) for patterns.

## 📖 API Browsing

### View rustdoc locally

```bash
cargo doc --no-deps --open
```

Docs include:
- `backyard` (main crate) - Overview + feature guide
- `backyard_core::Job` trait - Full trait documentation
- `backyard_core::Queue` trait - Backend interface
- All error types and options

### Key Traits Users Implement

1. **`Job` trait**
   - Implement `execute(self, ctx) -> Result<()>`
   - Set `const NAME: &'static str`

2. **Optional: `Job::options()`**
   - Override default queue name, retries, priority

## 🎯 Common Patterns

### Scheduled Jobs

```rust
let req = EnqueueRequest {
    scheduled_at: Utc::now() + Duration::hours(1),
    // ...
};
```

### High-Priority Jobs

```rust
let req = EnqueueRequest {
    queue: "critical".to_string(),
    priority: 10,  // Higher = more important
    // ...
};
```

### Error Handling

```rust
#[async_trait]
impl Job for MyJob {
    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        if something_failed {
            return Err(BackyardError::Execution("reason".into()));
        }
        Ok(())
    }
}
```

Failures auto-retry with exponential backoff.

### Enqueue from Job

```rust
#[async_trait]
impl Job for ProcessOrder {
    async fn execute(self, ctx: &JobContext) -> Result<()> {
        // Do work...
        
        // Enqueue follow-up
        let email = SendConfirmation { /* ... */ };
        ctx.enqueue(email, JobOptions::default()).await?;
        
        Ok(())
    }
}
```

## 🚨 Troubleshooting

### "Job type not registered"

Ensure jobs are in the same binary as the worker.

### "Database locked"

Increase `max_connections` in SQLite config:

```rust
SqliteConfig {
    max_connections: 10,
    // ...
}
```

### Jobs not executing

Check:
1. Worker is running: `WorkerBuilder::new(queue).run().await?`
2. Queue names match: Enqueue to `"default"`, watch `["default"]`
3. Job type registered: `SendEmail::NAME == "SendEmail"`

See [TESTING.md](TESTING.md) for debugging strategies.

## 📋 What's Not Yet in v0.1

- Redis backend is stubbed (architecture designed, implementation pending)
- Web UI endpoints designed but not implemented
- Cron scheduler plumbing in place but not fully wired

**These will ship in v0.2+ but SQLite backend is production-ready now.**

## 🔗 Dependency Tree

```
backyard (facade)
├── backyard-core (traits, no backend deps)
├── backyard-sqlite (SQLite impl)
├── backyard-redis (Redis impl, optional)
└── backyard-macros (#[derive(Job)])
```

Users only depend on `backyard`. Everything else is internal.

## 🎓 Learning Path

1. **First time?** → Read [USAGE.md](USAGE.md)
2. **Want examples?** → Check [examples/](examples/)
3. **How to test?** → See [TESTING.md](TESTING.md)
4. **API reference?** → Run `cargo doc --open`
5. **Understand architecture?** → Read [CLAUDE.md](CLAUDE.md)
6. **Deep dive?** → [README.md](README.md)

## 📦 Publishing Status

Backyard is **ready to publish to crates.io**:

- ✅ Proper Cargo.toml metadata
- ✅ Module documentation
- ✅ Examples
- ✅ Tests
- ✅ User guides
- ✅ Working integration tests

## 🤝 Contributing

Issues/PRs welcome at: https://github.com/user/backyard

See [CLAUDE.md](CLAUDE.md) for development notes.

---

**Ready to use? Start with [USAGE.md](USAGE.md)!** 🚀
