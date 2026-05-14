/// Quick test to verify backyard works end-to-end
///
/// Run with: cargo run --example quick_test --features sqlite
///
/// This example:
/// 1. Creates a real SQLite database
/// 2. Enqueues jobs manually
/// 3. Processes them with the worker pool
/// 4. Validates that jobs executed

use backyard::{Job, JobContext, Result, WorkerBuilder, SqliteQueue, SqliteConfig};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// Track what executed
lazy_static::lazy_static! {
    static ref EXECUTED_JOBS: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

#[derive(Job, Serialize, Deserialize, Clone)]
struct TestJob {
    message: String,
    should_fail: bool,
}

#[async_trait]
impl Job for TestJob {
    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        if self.should_fail {
            return Err(backyard::BackyardError::Execution("Intentional failure".to_string()));
        }

        EXECUTED_JOBS.lock().await.push(self.message.clone());
        println!("✓ Executed: {}", self.message);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Backyard Quick Test\n");

    // Setup: Create database
    let config = SqliteConfig {
        database_url: "sqlite://backyard_test.db".to_string(),
        poll_interval: std::time::Duration::from_millis(100),
        max_connections: 2,
    };

    let queue = SqliteQueue::new(config).await?;
    let queue = Arc::new(queue);

    println!("✓ Database initialized\n");

    // Test 1: Enqueue jobs
    println!("📝 Enqueueing jobs...");
    {
        let q = queue.clone();
        for i in 0..3 {
            let job = TestJob {
                message: format!("Job #{}", i),
                should_fail: false,
            };

            let payload = serde_json::to_vec(&job)?;
            let req = backyard::queue::EnqueueRequest {
                job_type: TestJob::NAME.to_string(),
                queue: "default".to_string(),
                payload,
                max_retries: 3,
                priority: 0,
                scheduled_at: chrono::Utc::now(),
            };

            let job_id = q.push(req).await?;
            println!("  → Job {} enqueued", i);
        }
    }

    // Test 2: Process jobs with timeout
    println!("\n⚙️  Processing jobs...");
    let queue_clone = queue.clone();
    let worker_handle = tokio::spawn(async move {
        let builder = WorkerBuilder::new(*queue_clone)
            .concurrency(2)
            .queues(["default"]);

        // Run worker for max 10 seconds
        tokio::select! {
            result = builder.run() => result,
            _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
                println!("  ⏱️  Worker timeout (jobs should be done by now)");
                Ok(())
            }
        }
    });

    // Give worker time to process
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Test 3: Verify execution
    println!("\n✅ Results:");
    let executed = EXECUTED_JOBS.lock().await;
    println!("  Executed {} jobs", executed.len());
    for job in executed.iter() {
        println!("    • {}", job);
    }

    if executed.len() == 3 {
        println!("\n🎉 All tests passed!");
    } else {
        println!("\n⚠️  Expected 3 jobs, got {}", executed.len());
    }

    Ok(())
}
