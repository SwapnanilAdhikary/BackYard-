use crate::{
    error::Result,
    job::{JobContext, RawJob},
    queue::Queue,
    registry::build_dispatch_table,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn, Instrument};

pub type HandlerFn = fn(&[u8], JobContext) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub queues: Vec<String>,
    pub concurrency: usize,
    pub poll_interval: std::time::Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            queues: vec!["default".into()],
            concurrency: 10,
            poll_interval: std::time::Duration::from_millis(500),
        }
    }
}

pub struct WorkerPool {
    queue: Arc<dyn Queue>,
    config: WorkerConfig,
    dispatch: HashMap<&'static str, HandlerFn>,
    shutdown: CancellationToken,
}

impl WorkerPool {
    pub fn new(queue: Arc<dyn Queue>, config: WorkerConfig) -> Self {
        Self {
            queue,
            config,
            dispatch: build_dispatch_table(),
            shutdown: CancellationToken::new(),
        }
    }

    pub fn shutdown_token(&self) -> CancellationToken {
        self.shutdown.child_token()
    }

    pub async fn run(self) -> Result<()> {
        let (tx, rx): (mpsc::Sender<RawJob>, mpsc::Receiver<RawJob>) =
            mpsc::channel(self.config.concurrency * 2);
        let rx = Arc::new(tokio::sync::Mutex::new(rx));

        let mut handles = vec![];
        for worker_id in 0..self.config.concurrency {
            let rx = rx.clone();
            let queue = self.queue.clone();
            let dispatch = self.dispatch.clone();
            let shutdown = self.shutdown.clone();
            let handle = tokio::spawn(async move {
                Self::worker_loop(worker_id.to_string(), rx, queue, dispatch, shutdown).await
            });
            handles.push(handle);
        }

        let fetch_shutdown = self.shutdown.clone();
        let queue = self.queue.clone();
        let queues: Vec<String> = self.config.queues.clone();
        let poll_interval = self.config.poll_interval;

        tokio::spawn(async move {
            Self::fetch_loop(queue, queues, tx, fetch_shutdown, poll_interval).await
        });

        futures::future::join_all(handles).await;
        Ok(())
    }

    async fn fetch_loop(
        queue: Arc<dyn Queue>,
        queues: Vec<String>,
        tx: mpsc::Sender<RawJob>,
        shutdown: CancellationToken,
        poll_interval: std::time::Duration,
    ) {
        let queue_refs: Vec<&str> = queues.iter().map(|s| s.as_str()).collect();
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                result = queue.pop(&queue_refs) => {
                    match result {
                        Ok(Some(job)) => {
                            let _ = tx.send(job).await;
                        }
                        Ok(None) => {
                            tokio::time::sleep(poll_interval).await;
                        }
                        Err(e) => {
                            error!("fetch error: {e}");
                            tokio::time::sleep(poll_interval).await;
                        }
                    }
                }
            }
        }
    }

    async fn worker_loop(
        worker_id: String,
        rx: Arc<tokio::sync::Mutex<mpsc::Receiver<RawJob>>>,
        queue: Arc<dyn Queue>,
        dispatch: HashMap<&'static str, HandlerFn>,
        shutdown: CancellationToken,
    ) {
        loop {
            let job: Option<RawJob> = {
                let mut rx = rx.lock().await;
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    job = rx.recv() => job
                }
            };

            let job = match job {
                Some(j) => j,
                None => break,
            };

            let ctx = JobContext {
                queue: queue.clone(),
                worker_id: worker_id.clone(),
            };

            match dispatch.get(job.job_type.as_str()) {
                None => {
                    error!(job_type = %job.job_type, "no handler registered");
                    let _ = queue.fail(job.id, "no handler registered").await;
                }
                Some(handler) => {
                    let span = tracing::info_span!(
                        "execute_job",
                        job_id = %job.id,
                        job_type = %job.job_type,
                        queue = %job.queue,
                        attempt = job.attempts,
                    );
                    let result = handler(&job.payload, ctx.clone()).instrument(span).await;
                    match result {
                        Ok(()) => {
                            info!(job_id = %job.id, "job succeeded");
                            let _ = queue.ack(job.id).await;
                        }
                        Err(e) => {
                            warn!(job_id = %job.id, error = %e, "job failed");
                            if job.attempts >= job.max_retries {
                                let _ = queue.fail(job.id, &e.to_string()).await;
                            } else {
                                let retry_at = crate::retry::next_retry_at(job.attempts);
                                let _ = queue.retry(job.id, retry_at).await;
                            }
                        }
                    }
                }
            }
        }
    }
}
