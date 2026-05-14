use backyard::{Job, WorkerBuilder, SqliteQueue, SqliteConfig, JobContext, Result};
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
        println!("Sending email to: {} with subject: {}", self.to, self.subject);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = SqliteConfig {
        database_url: "sqlite://backyard_example.db".to_string(),
        ..Default::default()
    };

    let queue = SqliteQueue::new(config).await?;
    let queue = std::sync::Arc::new(queue);

    let enqueue_queue = queue.clone();
    tokio::spawn(async move {
        for i in 0..5 {
            let job = SendEmail {
                to: format!("user{}@example.com", i),
                subject: format!("Hello {}", i),
            };

            let req = backyard::queue::EnqueueRequest {
                job_type: SendEmail::NAME.to_string(),
                queue: "default".to_string(),
                payload: serde_json::to_vec(&job).unwrap(),
                max_retries: 3,
                priority: 0,
                scheduled_at: chrono::Utc::now(),
            };

            match enqueue_queue.push(req).await {
                Ok(id) => println!("Enqueued job {}", id),
                Err(e) => eprintln!("Failed to enqueue: {}", e),
            }
        }
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    WorkerBuilder::new(*queue)
        .concurrency(2)
        .queues(["default"])
        .run()
        .await?;

    Ok(())
}
