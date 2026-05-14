CREATE TABLE IF NOT EXISTS jobs (
    id TEXT NOT NULL PRIMARY KEY,
    queue TEXT NOT NULL DEFAULT 'default',
    job_type TEXT NOT NULL,
    payload BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','running','failed','dead')),
    attempts INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    priority INTEGER NOT NULL DEFAULT 0,
    scheduled_at TEXT NOT NULL,
    locked_by TEXT,
    locked_at TEXT,
    error TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_queue_status_scheduled
    ON jobs(queue, status, scheduled_at);

CREATE INDEX IF NOT EXISTS idx_jobs_status
    ON jobs(status);
