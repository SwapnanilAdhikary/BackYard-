use async_trait::async_trait;
/// Real user test: Actually execute jobs end-to-end
/// Run with: cargo test --test user_test -- --nocapture
use backyard_core::{Job, JobContext, Queue, Result};
use backyard_sqlite::{SqliteConfig, SqliteQueue};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
struct PrintMessage {
    message: String,
}

#[async_trait]
impl Job for PrintMessage {
    const NAME: &'static str = "PrintMessage";

    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("📨 Job executed: {}", self.message);
        Ok(())
    }
}

#[tokio::test]
async fn user_test_enqueue_and_pop() -> Result<()> {
    println!("\n=== Test 1: Enqueue and Pop Job ===\n");

    let config = SqliteConfig {
        database_url: "sqlite://test_enqueue.db".to_string(),
        ..Default::default()
    };

    let queue = Arc::new(SqliteQueue::new(config).await?);

    // Create a job
    let job = PrintMessage {
        message: "Hello from backyard!".to_string(),
    };

    // Serialize it
    let payload = serde_json::to_vec(&job)?;
    println!("✓ Job serialized: {} bytes", payload.len());

    // Enqueue it
    let req = backyard_core::queue::EnqueueRequest {
        job_type: PrintMessage::NAME.to_string(),
        queue: "default".to_string(),
        payload,
        max_retries: 3,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };

    let job_id = queue.push(req).await?;
    println!("✓ Job enqueued with ID: {}", job_id);

    // Pop it back
    if let Some(popped) = queue.pop(&["default"]).await? {
        println!(
            "✓ Job popped: type={}, attempts={}",
            popped.job_type, popped.attempts
        );

        // Deserialize
        let parsed: PrintMessage = serde_json::from_slice(&popped.payload)?;
        println!("✓ Job deserialized: {}", parsed.message);

        // Execute it
        let ctx = JobContext {
            queue: queue.clone(),
            worker_id: "test-worker".to_string(),
        };
        parsed.execute(&ctx).await?;

        // Acknowledge
        queue.ack(popped.id).await?;
        println!("✓ Job acknowledged\n");

        Ok(())
    } else {
        println!("✗ No job found!");
        Err(backyard_core::BackyardError::NotFound("job".to_string()))
    }
}

#[tokio::test]
async fn user_test_multiple_jobs() -> Result<()> {
    println!("\n=== Test 2: Multiple Jobs ===\n");

    let config = SqliteConfig {
        database_url: "sqlite://test_multiple.db".to_string(),
        ..Default::default()
    };

    let queue = Arc::new(SqliteQueue::new(config).await?);

    // Enqueue 5 jobs
    for i in 0..5 {
        let job = PrintMessage {
            message: format!("Job #{}", i),
        };

        let payload = serde_json::to_vec(&job)?;
        let req = backyard_core::queue::EnqueueRequest {
            job_type: PrintMessage::NAME.to_string(),
            queue: "default".to_string(),
            payload,
            max_retries: 3,
            priority: 0,
            scheduled_at: chrono::Utc::now(),
        };

        queue.push(req).await?;
    }
    println!("✓ Enqueued 5 jobs");

    // Check queue depth
    let depths = queue.queue_depths().await?;
    println!("✓ Queue depths: {:?}", depths);

    // Pop and process all
    let mut count = 0;
    while let Some(job) = queue.pop(&["default"]).await? {
        let parsed: PrintMessage = serde_json::from_slice(&job.payload)?;
        println!("  → {}", parsed.message);

        let ctx = JobContext {
            queue: queue.clone(),
            worker_id: "test-worker".to_string(),
        };
        parsed.execute(&ctx).await?;
        queue.ack(job.id).await?;
        count += 1;
    }

    println!("✓ Processed {} jobs\n", count);
    assert_eq!(count, 5);
    Ok(())
}

#[tokio::test]
async fn user_test_retry_logic() -> Result<()> {
    println!("\n=== Test 3: Failure and Retry ===\n");

    #[derive(Serialize, Deserialize)]
    struct FailingJob {
        attempt: u32,
    }

    #[async_trait]
    impl Job for FailingJob {
        const NAME: &'static str = "FailingJob";

        async fn execute(self, _ctx: &JobContext) -> Result<()> {
            if self.attempt < 2 {
                Err(backyard_core::BackyardError::Execution(
                    "Not ready yet".to_string(),
                ))
            } else {
                println!("✓ Finally succeeded!");
                Ok(())
            }
        }
    }

    let config = SqliteConfig {
        database_url: "sqlite://test_retry.db".to_string(),
        ..Default::default()
    };

    let queue = Arc::new(SqliteQueue::new(config).await?);

    // Enqueue job
    let job = FailingJob { attempt: 0 };
    let payload = serde_json::to_vec(&job)?;
    let req = backyard_core::queue::EnqueueRequest {
        job_type: FailingJob::NAME.to_string(),
        queue: "default".to_string(),
        payload,
        max_retries: 5,
        priority: 0,
        scheduled_at: chrono::Utc::now(),
    };

    let _job_id = queue.push(req).await?;
    println!("✓ Enqueued failing job");

    // First attempt - should fail
    let popped = queue.pop(&["default"]).await?.unwrap();
    println!("  Attempt 1: popped with attempts={}", popped.attempts);

    let parsed: FailingJob = serde_json::from_slice(&popped.payload)?;
    let ctx = JobContext {
        queue: queue.clone(),
        worker_id: "test".to_string(),
    };

    match parsed.execute(&ctx).await {
        Ok(_) => println!("✗ Should have failed"),
        Err(_) => {
            println!("  → Failed (expected)");
            // Retry
            let retry_at = backyard_core::retry::next_retry_at(popped.attempts);
            queue.retry(popped.id, retry_at).await?;
            println!("  → Scheduled retry at: {}", retry_at);
        }
    }

    // Check updated job
    let job_after = queue.get(popped.id).await?.unwrap();
    println!(
        "✓ Job rescheduled: attempts={}, scheduled_at={}",
        job_after.attempts, job_after.scheduled_at
    );

    assert!(job_after.attempts > 0);
    println!();
    Ok(())
}

#[tokio::test]
async fn user_test_job_inspection() -> Result<()> {
    println!("\n=== Test 4: Job Inspection ===\n");

    let config = SqliteConfig {
        database_url: "sqlite://test_inspect.db".to_string(),
        ..Default::default()
    };

    let queue = SqliteQueue::new(config).await?;

    // Enqueue multiple jobs
    for i in 0..3 {
        let job = PrintMessage {
            message: format!("Message {}", i),
        };
        let payload = serde_json::to_vec(&job)?;
        let req = backyard_core::queue::EnqueueRequest {
            job_type: PrintMessage::NAME.to_string(),
            queue: "test".to_string(),
            payload,
            max_retries: 3,
            priority: i as i32,
            scheduled_at: chrono::Utc::now(),
        };
        queue.push(req).await?;
    }

    // List jobs
    let jobs = queue.list("test", None, 10, 0).await?;
    println!("✓ Found {} jobs in 'test' queue:", jobs.len());
    for job in &jobs {
        println!(
            "  • ID: {}, type: {}, priority: {}",
            job.id, job.job_type, job.attempts
        );
    }

    // Get specific job
    if let Some(first) = jobs.first() {
        let fetched = queue.get(first.id).await?;
        assert!(fetched.is_some());
        println!("✓ Successfully retrieved job by ID");
    }

    println!();
    Ok(())
}
