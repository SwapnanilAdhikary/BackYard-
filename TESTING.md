# Testing Guide for Backyard

This guide covers how to test your jobs, ensure they're processed correctly, and validate your queue setup.

## Quick Start: Manual Testing

### 1. Define a Job

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

### 2. Test Job Serialization

```rust
#[test]
fn test_job_serialization() {
    let job = SendEmail {
        to: "user@example.com".to_string(),
        subject: "Hello".to_string(),
    };
    
    let serialized = serde_json::to_vec(&job).unwrap();
    let deserialized: SendEmail = serde_json::from_slice(&serialized).unwrap();
    
    assert_eq!(deserialized.to, job.to);
}
```

### 3. Test Job Execution

```rust
#[tokio::test]
async fn test_job_execution() {
    let job = SendEmail {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
    };
    
    let ctx = JobContext {
        queue: Arc::new(mock_queue()),  // Or real queue
        worker_id: "test-worker".to_string(),
    };
    
    let result = job.execute(&ctx).await;
    assert!(result.is_ok());
}
```

## Integration Testing: Full Workflow

### Setup Queue for Testing

```rust
use backyard::{SqliteQueue, SqliteConfig};

#[tokio::test]
async fn test_enqueue_and_process() {
    // Use a real file-based database for tests
    let config = SqliteConfig {
        database_url: "sqlite://test.db".to_string(),
        ..Default::default()
    };
    
    let queue = SqliteQueue::new(config).await.unwrap();
    
    // Rest of test...
}
```

### Enqueue a Job

```rust
// Create the job
let job = SendEmail {
    to: "alice@example.com".to_string(),
    subject: "Welcome!".to_string(),
};

// Serialize it
let payload = serde_json::to_vec(&job).unwrap();

// Enqueue via the request API
let req = backyard::queue::EnqueueRequest {
    job_type: SendEmail::NAME.to_string(),
    queue: "default".to_string(),
    payload,
    max_retries: 3,
    priority: 0,
    scheduled_at: chrono::Utc::now(),
};

let job_id = queue.push(req).await.unwrap();
println!("Enqueued: {}", job_id);
```

### Pop and Verify Job

```rust
// Pop the job
if let Some(job) = queue.pop(&["default"]).await.unwrap() {
    // Verify metadata
    assert_eq!(job.job_type, "SendEmail");
    assert_eq!(job.queue, "default");
    assert_eq!(job.attempts, 0);
    
    // Deserialize and verify payload
    let parsed: SendEmail = serde_json::from_slice(&job.payload).unwrap();
    assert_eq!(parsed.to, "alice@example.com");
    
    // Mark as complete
    queue.ack(job.id).await.unwrap();
}
```

## Testing Failure & Retry

### Test Job Failure

```rust
#[derive(Job, Serialize, Deserialize)]
struct ProcessPayment {
    amount: i32,
}

#[async_trait]
impl Job for ProcessPayment {
    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        if self.amount < 0 {
            return Err(BackyardError::Execution("Invalid amount".to_string()));
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_failure_and_retry() {
    let queue = get_test_queue().await;
    
    // Enqueue a job that will fail
    let job = ProcessPayment { amount: -100 };
    let payload = serde_json::to_vec(&job).unwrap();
    
    let req = EnqueueRequest {
        job_type: ProcessPayment::NAME.to_string(),
        queue: "default".to_string(),
        payload,
        max_retries: 3,
        priority: 0,
        scheduled_at: Utc::now(),
    };
    
    let job_id = queue.push(req).await.unwrap();
    
    // Pop it
    let popped = queue.pop(&["default"]).await.unwrap().unwrap();
    assert_eq!(popped.attempts, 0);
    
    // Execute (will fail)
    let parsed: ProcessPayment = serde_json::from_slice(&popped.payload).unwrap();
    let result = parsed.execute(&ctx).await;
    assert!(result.is_err());
    
    // Retry
    let retry_at = backyard::retry::next_retry_at(popped.attempts);
    queue.retry(popped.id, retry_at).await.unwrap();
    
    // Verify it's scheduled
    let rescheduled = queue.get(popped.id).await.unwrap().unwrap();
    assert_eq!(rescheduled.attempts, 1);
    assert!(rescheduled.scheduled_at > Utc::now());
}
```

### Test Dead Letter Queue (Max Retries)

```rust
#[tokio::test]
async fn test_dlq() {
    let queue = get_test_queue().await;
    
    // Enqueue and pop
    let job = ProcessPayment { amount: -100 };
    // ... enqueue, pop, fail multiple times ...
    
    // After max retries
    let popped = queue.pop(&["default"]).await.unwrap().unwrap();
    if popped.attempts >= popped.max_retries {
        queue.fail(popped.id, "Max retries exceeded").await.unwrap();
    }
    
    // Verify it's in DLQ (deleted from pending, marked as dead)
    let dead_job = queue.get(popped.id).await.unwrap();
    assert!(dead_job.is_some());
    assert!(dead_job.unwrap().error.is_some());
}
```

## Testing Delayed Jobs

```rust
#[tokio::test]
async fn test_scheduled_jobs() {
    let queue = get_test_queue().await;
    
    let job = SendEmail { /* ... */ };
    let payload = serde_json::to_vec(&job).unwrap();
    
    let future_time = Utc::now() + chrono::Duration::hours(1);
    
    let req = EnqueueRequest {
        job_type: SendEmail::NAME.to_string(),
        queue: "default".to_string(),
        payload,
        max_retries: 3,
        priority: 0,
        scheduled_at: future_time,  // Future time!
    };
    
    queue.push(req).await.unwrap();
    
    // Job should NOT be available yet
    let popped = queue.pop(&["default"]).await.unwrap();
    assert!(popped.is_none(), "Job shouldn't be ready yet");
}
```

## Testing Multiple Queues

```rust
#[tokio::test]
async fn test_queue_priority() {
    let queue = get_test_queue().await;
    
    // Enqueue to different queues
    enqueue_job(&queue, "default", SendEmail { /* ... */ }).await;
    enqueue_job(&queue, "critical", SendEmail { /* ... */ }).await;
    enqueue_job(&queue, "default", SendEmail { /* ... */ }).await;
    
    // Check depths
    let depths = queue.queue_depths().await.unwrap();
    assert_eq!(depths.get("default"), Some(&2));
    assert_eq!(depths.get("critical"), Some(&1));
    
    // Pop from critical first
    let job = queue.pop(&["critical", "default"]).await.unwrap().unwrap();
    assert_eq!(job.queue, "critical");
}
```

## Running Tests

### Run all tests
```bash
cargo test --lib
```

### Run specific test
```bash
cargo test test_enqueue_and_process
```

### Run with output
```bash
cargo test -- --nocapture
```

### Run tests in a file
```bash
cargo test --test basic
```

### Run examples as integration tests
```bash
cargo run --example testing_guide
```

## Best Practices

### 1. Use Real Databases for Integration Tests

```rust
// ✅ Good: Real database, easier to debug
let queue = SqliteQueue::new(SqliteConfig {
    database_url: "sqlite://test.db".to_string(),
    ..Default::default()
}).await?;

// ❌ Avoid: In-memory DBs don't persist across connections
let queue = SqliteQueue::new(SqliteConfig {
    database_url: "sqlite://:memory:".to_string(),  // Problematic
    ..Default::default()
}).await?;
```

### 2. Clean Up After Tests

```rust
#[tokio::test]
async fn test_something() {
    // Setup
    let queue = SqliteQueue::new(config).await?;
    
    // Test...
    
    // Cleanup
    std::fs::remove_file("sqlite://test.db").ok();
}
```

### 3. Test Serialization Explicitly

```rust
#[test]
fn test_job_serialization() {
    let original = MyJob { /* ... */ };
    let serialized = serde_json::to_vec(&original).unwrap();
    let deserialized: MyJob = serde_json::from_slice(&serialized).unwrap();
    
    // Verify all fields match
    assert_eq!(deserialized.field1, original.field1);
}
```

### 4. Test Error Cases

```rust
#[tokio::test]
async fn test_job_with_invalid_input() {
    let job = MyJob { input: "invalid" };
    let ctx = JobContext { /* ... */ };
    
    let result = job.execute(&ctx).await;
    assert!(result.is_err(), "Should handle invalid input");
}
```

### 5. Use Fixtures for Complex Setup

```rust
async fn setup_queue() -> SqliteQueue {
    SqliteQueue::new(SqliteConfig {
        database_url: "sqlite://test.db".to_string(),
        ..Default::default()
    }).await.unwrap()
}

#[tokio::test]
async fn test_with_fixture() {
    let queue = setup_queue().await;
    // Test...
}
```

## Manual Testing: Run Examples

```bash
# Run the quick test
cargo run --example quick_test --features sqlite

# Run the comprehensive testing guide
cargo run --example testing_guide

# Run the basic example
cargo run --example basic_sqlite
```

## Debugging Failed Tests

### Print job state
```rust
let job = queue.pop(&["default"]).await?;
if let Some(j) = job {
    println!("Job: {:#?}", j);  // Pretty print
    println!("Payload: {:?}", String::from_utf8_lossy(&j.payload));
}
```

### Check queue state
```rust
let depths = queue.queue_depths().await?;
println!("Queue depths: {:#?}", depths);

let jobs = queue.list("default", None, 100, 0).await?;
println!("Jobs in queue: {:#?}", jobs);
```

### Test with logging
```rust
#[tokio::test]
async fn test_with_logs() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    
    // Your test...
}
```

## Next Steps

Once you're confident your jobs work:

1. **Add to CI**: Include tests in GitHub Actions
2. **Load test**: Enqueue many jobs, measure throughput
3. **Failure scenarios**: Test network failures, crashes
4. **Monitoring**: Add metrics, check job latency
