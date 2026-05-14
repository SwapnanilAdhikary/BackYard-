use async_trait::async_trait;
use backyard_core::queue::EnqueueRequest;
use backyard_core::{BackyardError, Queue, RawJob, Result};
use chrono::Utc;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;

pub struct SqliteQueue {
    pool: SqlitePool,
    worker_id: String,
}

impl SqliteQueue {
    pub async fn new(config: crate::config::SqliteConfig) -> Result<Self> {
        // Use URL parsing so `sqlite://:memory:` gets `shared_cache` + correct in-memory URI.
        // `.filename(":memory:")` alone gives each pool connection a private DB (no `jobs` table).
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(&config.database_url)
            .map_err(|e| BackyardError::Backend(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| BackyardError::Backend(e.to_string()))?;

        let _migration_sql = include_str!("../migrations/20260514000000_initial.sql");

        let create_table = r#"
        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT NOT NULL PRIMARY KEY,
            queue TEXT NOT NULL DEFAULT 'default',
            job_type TEXT NOT NULL,
            payload BLOB NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            max_retries INTEGER NOT NULL DEFAULT 3,
            priority INTEGER NOT NULL DEFAULT 0,
            scheduled_at TEXT NOT NULL,
            locked_by TEXT,
            locked_at TEXT,
            error TEXT,
            created_at TEXT NOT NULL
        )"#;

        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| BackyardError::Backend(format!("Failed to get connection: {}", e)))?;

        sqlx::raw_sql(create_table)
            .execute(&mut *conn)
            .await
            .map_err(|e| BackyardError::Backend(format!("Create table failed: {}", e)))?;

        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_jobs_queue_status_scheduled ON jobs(queue, status, scheduled_at)")
            .execute(&mut *conn)
            .await
            .map_err(|e| BackyardError::Backend(format!("Create index 1 failed: {}", e)))?;

        sqlx::raw_sql("CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)")
            .execute(&mut *conn)
            .await
            .map_err(|e| BackyardError::Backend(format!("Create index 2 failed: {}", e)))?;

        Ok(Self {
            pool,
            worker_id: Uuid::new_v4().to_string(),
        })
    }
}

#[async_trait]
impl Queue for SqliteQueue {
    async fn push(&self, req: EnqueueRequest) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let scheduled_at_str = req.scheduled_at.to_rfc3339();
        let created_at_str = now.to_rfc3339();

        sqlx::query(
            r#"INSERT INTO jobs
               (id, queue, job_type, payload, status, attempts, max_retries, priority, scheduled_at, created_at)
               VALUES (?, ?, ?, ?, 'pending', 0, ?, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(&req.queue)
        .bind(&req.job_type)
        .bind(&req.payload)
        .bind(req.max_retries as i64)
        .bind(req.priority as i64)
        .bind(scheduled_at_str)
        .bind(created_at_str)
        .execute(&self.pool)
        .await
        .map_err(|e| BackyardError::Backend(e.to_string()))?;

        Ok(id)
    }

    async fn pop(&self, queues: &[&str]) -> Result<Option<RawJob>> {
        if queues.is_empty() {
            return Ok(None);
        }

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| BackyardError::Backend(e.to_string()))?;

        let now = Utc::now().to_rfc3339();
        let placeholders = (0..queues.len())
            .map(|i| format!("${}", i + 2))
            .collect::<Vec<_>>()
            .join(",");

        let query_str = format!(
            r#"SELECT id, queue, job_type, payload, attempts, max_retries, scheduled_at, created_at, error
               FROM jobs
               WHERE status = 'pending'
                 AND scheduled_at <= ?1
                 AND queue IN ({})
               ORDER BY priority ASC, created_at ASC
               LIMIT 1"#,
            placeholders
        );

        let mut query = sqlx::query(&query_str).bind(&now);
        for queue in queues {
            query = query.bind(queue);
        }

        let row = query
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| BackyardError::Backend(e.to_string()))?;

        if let Some(row) = row {
            let id_str: String = row.get("id");
            let locked_at_str = Utc::now().to_rfc3339();

            sqlx::query("UPDATE jobs SET status='running', locked_by=?, locked_at=? WHERE id=?")
                .bind(&self.worker_id)
                .bind(&locked_at_str)
                .bind(&id_str)
                .execute(&mut *tx)
                .await
                .map_err(|e| BackyardError::Backend(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| BackyardError::Backend(e.to_string()))?;

            let id = Uuid::parse_str(&id_str).map_err(|e| BackyardError::Backend(e.to_string()))?;
            let queue: String = row.get("queue");
            let job_type: String = row.get("job_type");
            let payload: Vec<u8> = row.get("payload");
            let attempts: i64 = row.get("attempts");
            let max_retries: i64 = row.get("max_retries");
            let scheduled_at_str: String = row.get("scheduled_at");
            let created_at_str: String = row.get("created_at");
            let error: Option<String> = row.get("error");

            let scheduled_at = chrono::DateTime::parse_from_rfc3339(&scheduled_at_str)
                .map_err(|e| BackyardError::Backend(e.to_string()))?
                .with_timezone(&Utc);
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| BackyardError::Backend(e.to_string()))?
                .with_timezone(&Utc);

            Ok(Some(RawJob {
                id,
                job_type,
                queue,
                payload,
                attempts: attempts as u32,
                max_retries: max_retries as u32,
                scheduled_at,
                created_at,
                error,
            }))
        } else {
            tx.rollback().await.ok();
            Ok(None)
        }
    }

    async fn ack(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM jobs WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| BackyardError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn fail(&self, id: Uuid, err: &str) -> Result<()> {
        sqlx::query(
            "UPDATE jobs SET status='dead', error=?, locked_by=NULL, locked_at=NULL WHERE id=?",
        )
        .bind(err)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| BackyardError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn retry(&self, id: Uuid, retry_at: chrono::DateTime<Utc>) -> Result<()> {
        let retry_at_str = retry_at.to_rfc3339();
        sqlx::query(
            r#"UPDATE jobs
               SET status='pending',
                   attempts = attempts + 1,
                   scheduled_at = ?,
                   locked_by = NULL,
                   locked_at = NULL
               WHERE id = ?"#,
        )
        .bind(retry_at_str)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| BackyardError::Backend(e.to_string()))?;
        Ok(())
    }

    async fn get(&self, id: Uuid) -> Result<Option<RawJob>> {
        let row = sqlx::query(
            r#"SELECT id, queue, job_type, payload, attempts, max_retries, scheduled_at, created_at, error
               FROM jobs WHERE id = ?"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| BackyardError::Backend(e.to_string()))?;

        Ok(row.and_then(|r| {
            let id_str: String = r.get("id");
            let id = Uuid::parse_str(&id_str).ok()?;
            let queue: String = r.get("queue");
            let job_type: String = r.get("job_type");
            let payload: Vec<u8> = r.get("payload");
            let attempts: i64 = r.get("attempts");
            let max_retries: i64 = r.get("max_retries");
            let scheduled_at_str: String = r.get("scheduled_at");
            let created_at_str: String = r.get("created_at");
            let error: Option<String> = r.get("error");

            let scheduled_at = chrono::DateTime::parse_from_rfc3339(&scheduled_at_str)
                .ok()?
                .with_timezone(&Utc);
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .ok()?
                .with_timezone(&Utc);

            Some(RawJob {
                id,
                job_type,
                queue,
                payload,
                attempts: attempts as u32,
                max_retries: max_retries as u32,
                scheduled_at,
                created_at,
                error,
            })
        }))
    }

    async fn list(
        &self,
        queue: &str,
        status: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<RawJob>> {
        let query_str = if let Some(_status) = status {
            format!(
                r#"SELECT id, queue, job_type, payload, attempts, max_retries, scheduled_at, created_at, error
                   FROM jobs
                   WHERE queue = ? AND status = ?
                   ORDER BY priority ASC, created_at ASC
                   LIMIT ? OFFSET ?"#
            )
        } else {
            format!(
                r#"SELECT id, queue, job_type, payload, attempts, max_retries, scheduled_at, created_at, error
                   FROM jobs
                   WHERE queue = ?
                   ORDER BY priority ASC, created_at ASC
                   LIMIT ? OFFSET ?"#
            )
        };

        let rows = if let Some(status) = status {
            sqlx::query(&query_str)
                .bind(queue)
                .bind(status)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
        } else {
            sqlx::query(&query_str)
                .bind(queue)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
        }
        .map_err(|e| BackyardError::Backend(e.to_string()))?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let id_str: String = r.get("id");
                let id = Uuid::parse_str(&id_str).ok()?;
                let queue: String = r.get("queue");
                let job_type: String = r.get("job_type");
                let payload: Vec<u8> = r.get("payload");
                let attempts: i64 = r.get("attempts");
                let max_retries: i64 = r.get("max_retries");
                let scheduled_at_str: String = r.get("scheduled_at");
                let created_at_str: String = r.get("created_at");
                let error: Option<String> = r.get("error");

                let scheduled_at = chrono::DateTime::parse_from_rfc3339(&scheduled_at_str)
                    .ok()?
                    .with_timezone(&Utc);
                let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .ok()?
                    .with_timezone(&Utc);

                Some(RawJob {
                    id,
                    job_type,
                    queue,
                    payload,
                    attempts: attempts as u32,
                    max_retries: max_retries as u32,
                    scheduled_at,
                    created_at,
                    error,
                })
            })
            .collect())
    }

    async fn queue_depths(&self) -> Result<HashMap<String, u64>> {
        let rows = sqlx::query(
            r#"SELECT queue, COUNT(*) as count
               FROM jobs
               WHERE status = 'pending' OR status = 'running'
               GROUP BY queue"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| BackyardError::Backend(e.to_string()))?;

        let mut depths = HashMap::new();
        for row in rows {
            let queue: String = row.get("queue");
            let count: i64 = row.get("count");
            depths.insert(queue, count as u64);
        }

        Ok(depths)
    }
}
