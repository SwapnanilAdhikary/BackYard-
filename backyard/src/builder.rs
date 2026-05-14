use backyard_core::{Queue, Result, WorkerConfig, WorkerPool};
use std::sync::Arc;

pub struct WorkerBuilder {
    queue: Arc<dyn Queue>,
    config: WorkerConfig,
}

impl WorkerBuilder {
    pub fn new(queue: impl Queue + 'static) -> Self {
        Self {
            queue: Arc::new(queue),
            config: WorkerConfig::default(),
        }
    }

    pub fn queues(mut self, queues: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.config.queues = queues.into_iter().map(Into::into).collect();
        self
    }

    pub fn concurrency(mut self, n: usize) -> Self {
        self.config.concurrency = n;
        self
    }

    pub async fn run(self) -> Result<()> {
        let pool = WorkerPool::new(self.queue, self.config);
        pool.run().await
    }
}
