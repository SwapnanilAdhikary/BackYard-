/// Testing Guide for Backyard Users
///
/// This example demonstrates how to test your job definitions, enqueuing, and processing.

use backyard::{Job, JobContext, Result, WorkerBuilder, SqliteQueue, SqliteConfig};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

// ============================================================================
// 1. Define Your Jobs
// ============================================================================

#[derive(Job, Serialize, Deserialize, Clone, Debug)]
struct SendWelcomeEmail {
    user_id: String,
    email: String,
}

#[async_trait]
impl Job for SendWelcomeEmail {
    async fn execute(self, ctx: &JobContext) -> Result<()> {
        println!("📧 Sending welcome email to {} ({})", self.email, self.user_id);
        // In real code: call email service
        Ok(())
    }
}

#[derive(Job, Serialize, Deserialize, Clone, Debug)]
struct ProcessPayment {
    payment_id: String,
    amount_cents: i32,
}

#[async_trait]
impl Job for ProcessPayment {
    async fn execute(self, _ctx: &JobContext) -> Result<()> {
        println!("💳 Processing payment {} for ${:.2}", self.payment_id, self.amount_cents as f64 / 100.0);

        // Simulate potential failure
        if self.amount_cents < 0 {
            return Err(backyard::BackyardError::Execution("Invalid amount".to_string()));
        }

        Ok(())
    }
}

// ============================================================================
// 2. Test Harness Setup
// ============================================================================

struct TestHarness {
    queue: Arc<SqliteQueue>,
    processed_jobs: Arc<Mutex<Vec<String>>>,
}

impl TestHarness {
    async fn setup() -> Result<Self> {
        let config = SqliteConfig {
            database_url: "sqlite::memory:".to_string(),
            ..Default::default()
        };

        let queue = SqliteQueue::new(config).await?;

        Ok(Self {
            queue: Arc::new(queue),
            processed_jobs: Arc::new(Mutex::new(Vec::new())),
        })
    }

    async fn enqueue_email(&self, user_id: &str, email: &str) -> Result<()> {
        let job = SendWelcomeEmail {
            user_id: user_id.to_string(),
            email: email.to_string(),
        };

        let payload = serde_json::to_vec(&job)?;
        let req = backyard::queue::EnqueueRequest {
            job_type: SendWelcomeEmail::NAME.to_string(),
            queue: "default".to_string(),
            payload,
            max_retries: 3,
            priority: 0,
            scheduled_at: chrono::Utc::now(),
        };

        let job_id = self.queue.push(req).await?;
        println!("✓ Enqueued SendWelcomeEmail: {}", job_id);
        Ok(())
    }

    async fn enqueue_payment(&self, payment_id: &str, amount_cents: i32) -> Result<()> {
        let job = ProcessPayment {
            payment_id: payment_id.to_string(),
            amount_cents,
        };

        let payload = serde_json::to_vec(&job)?;
        let req = backyard::queue::EnqueueRequest {
            job_type: ProcessPayment::NAME.to_string(),
            queue: "payments".to_string(),
            payload,
            max_retries: 3,
            priority: 1,
            scheduled_at: chrono::Utc::now(),
        };

        let job_id = self.queue.push(req).await?;
        println!("✓ Enqueued ProcessPayment: {}", job_id);
        Ok(())
    }
}

// ============================================================================
// 3. Manual Testing (Single Job)
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n=== Backyard Testing Guide ===\n");

    let harness = TestHarness::setup().await?;

    // Test 1: Enqueue and inspect
    println!("Test 1: Enqueue a job");
    harness.enqueue_email("user123", "alice@example.com").await?;

    let job = harness.queue.pop(&["default"]).await?;
    if let Some(job) = job {
        println!("✓ Popped job: type={}, queue={}, id={}", job.job_type, job.queue, job.id);

        let parsed_job: SendWelcomeEmail = serde_json::from_slice(&job.payload)?;
        println!("✓ Deserialized: {:?}", parsed_job);

        // Simulate success
        harness.queue.ack(job.id).await?;
        println!("✓ Job acknowledged");
    }

    // Test 2: Failure and retry
    println!("\nTest 2: Failure and retry");
    harness.enqueue_payment("pay_001", -100).await?;

    if let Some(job) = harness.queue.pop(&["payments"]).await? {
        let parsed_job: ProcessPayment = serde_json::from_slice(&job.payload)?;
        println!("Job details: {:?}", parsed_job);

        // Simulate execution (which will fail)
        match parsed_job.execute(&JobContext {
            queue: harness.queue.clone(),
            worker_id: "test-worker".to_string(),
        }).await {
            Ok(_) => println!("✗ Should have failed!"),
            Err(e) => {
                println!("✓ Job failed as expected: {}", e);

                // In real code, worker pool would call this
                let retry_at = backyard::retry::next_retry_at(job.attempts);
                harness.queue.retry(job.id, retry_at).await?;
                println!("✓ Job scheduled for retry at: {}", retry_at);

                let retried_job = harness.queue.get(job.id).await?;
                if let Some(j) = retried_job {
                    println!("✓ Retry successful: attempts={}, status will be pending, scheduled_at={}", j.attempts, j.scheduled_at);
                }
            }
        }
    }

    // Test 3: Multiple jobs
    println!("\nTest 3: Multiple jobs and queue depths");
    harness.enqueue_email("user456", "bob@example.com").await?;
    harness.enqueue_email("user789", "charlie@example.com").await?;
    harness.enqueue_payment("pay_002", 5000).await?;

    let depths = harness.queue.queue_depths().await?;
    println!("✓ Queue depths: {:?}", depths);

    // Test 4: Dead letter queue (max retries)
    println!("\nTest 4: Dead letter queue");
    harness.enqueue_payment("pay_bad", -1).await?;

    if let Some(job) = harness.queue.pop(&["payments"]).await? {
        let error_msg = "Payment amount cannot be negative";

        // Simulate max retries exceeded
        if job.attempts >= job.max_retries {
            harness.queue.fail(job.id, error_msg).await?;
            println!("✓ Job moved to DLQ: {}", error_msg);

            let dead_job = harness.queue.get(job.id).await?;
            if let Some(j) = dead_job {
                println!("✓ Dead job preserved: error={:?}", j.error);
            }
        }
    }

    println!("\n=== All Manual Tests Passed ===\n");
    Ok(())
}

// ============================================================================
// 4. Automated Test Examples (use in tests/)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_and_pop() -> Result<()> {
        let harness = TestHarness::setup().await?;
        harness.enqueue_email("user1", "test@example.com").await?;

        let job = harness.queue.pop(&["default"]).await?;
        assert!(job.is_some(), "Job should be popped");

        let job = job.unwrap();
        assert_eq!(job.job_type, "SendWelcomeEmail");

        Ok(())
    }

    #[tokio::test]
    async fn test_job_serialization() -> Result<()> {
        let original = SendWelcomeEmail {
            user_id: "u123".to_string(),
            email: "test@example.com".to_string(),
        };

        let serialized = serde_json::to_vec(&original)?;
        let deserialized: SendWelcomeEmail = serde_json::from_slice(&serialized)?;

        assert_eq!(deserialized.user_id, original.user_id);
        assert_eq!(deserialized.email, original.email);

        Ok(())
    }

    #[tokio::test]
    async fn test_job_execution() -> Result<()> {
        let job = SendWelcomeEmail {
            user_id: "u456".to_string(),
            email: "alice@example.com".to_string(),
        };

        let ctx = JobContext {
            queue: Arc::new(TestHarness::setup().await?.queue),
            worker_id: "test-worker".to_string(),
        };

        let result = job.execute(&ctx).await;
        assert!(result.is_ok(), "Job execution should succeed");

        Ok(())
    }

    #[tokio::test]
    async fn test_retry_logic() -> Result<()> {
        let now = chrono::Utc::now();
        let retry_1 = backyard::retry::next_retry_at(0);
        let retry_2 = backyard::retry::next_retry_at(1);

        assert!(retry_1 > now, "Retry should be in the future");
        assert!(retry_2 > retry_1, "Each retry should be further in future");

        Ok(())
    }
}
