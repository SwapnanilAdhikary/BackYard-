use backyard_core::queue::EnqueueRequest;
use backyard_core::{Queue, Result};
use backyard_sqlite::{SqliteConfig, SqliteQueue};
use chrono::Utc;

#[tokio::test]
async fn test_sqlite_push_and_pop() -> Result<()> {
    let config = SqliteConfig {
        database_url: "sqlite://:memory:".to_string(),
        ..Default::default()
    };

    let queue = SqliteQueue::new(config).await?;

    let req = EnqueueRequest {
        job_type: "TestJob".to_string(),
        queue: "default".to_string(),
        payload: vec![1, 2, 3],
        max_retries: 3,
        priority: 0,
        scheduled_at: Utc::now(),
    };

    let job_id = queue.push(req).await?;
    assert!(!job_id.is_nil(), "Job ID should not be nil");

    let job = queue.pop(&["default"]).await?;
    assert!(job.is_some(), "Should be able to pop the job");

    let job = job.unwrap();
    assert_eq!(job.job_type, "TestJob");
    assert_eq!(job.attempts, 0);

    queue.ack(job.id).await?;

    let job_after_ack = queue.get(job.id).await?;
    assert!(job_after_ack.is_none(), "Job should be deleted after ack");

    Ok(())
}

#[tokio::test]
async fn test_sqlite_fail() -> Result<()> {
    let config = SqliteConfig {
        database_url: "sqlite://:memory:".to_string(),
        ..Default::default()
    };

    let queue = SqliteQueue::new(config).await?;

    let req = EnqueueRequest {
        job_type: "TestJob".to_string(),
        queue: "default".to_string(),
        payload: vec![1, 2, 3],
        max_retries: 3,
        priority: 0,
        scheduled_at: Utc::now(),
    };

    let _job_id = queue.push(req).await?;
    let job = queue.pop(&["default"]).await?.unwrap();

    queue.fail(job.id, "Something went wrong").await?;

    let job_after = queue.get(job.id).await?;
    assert!(job_after.is_some());
    assert_eq!(
        job_after.unwrap().error.as_deref(),
        Some("Something went wrong")
    );

    Ok(())
}
