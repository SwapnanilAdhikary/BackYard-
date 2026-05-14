use chrono::{DateTime, Duration, Utc};
use std::time;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JobOptions {
    pub queue: String,
    pub delay: Option<time::Duration>,
    pub at: Option<DateTime<Utc>>,
    pub max_retries: u32,
    pub priority: i32,
}

impl Default for JobOptions {
    fn default() -> Self {
        Self {
            queue: "default".into(),
            delay: None,
            at: None,
            max_retries: 3,
            priority: 0,
        }
    }
}

impl JobOptions {
    pub fn queue(mut self, q: impl Into<String>) -> Self {
        self.queue = q.into();
        self
    }

    pub fn delay(mut self, d: time::Duration) -> Self {
        self.delay = Some(d);
        self
    }

    pub fn at(mut self, t: DateTime<Utc>) -> Self {
        self.at = Some(t);
        self
    }

    pub fn max_retries(mut self, n: u32) -> Self {
        self.max_retries = n;
        self
    }

    pub fn priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }

    pub fn scheduled_at(&self) -> DateTime<Utc> {
        if let Some(at) = self.at {
            return at;
        }
        if let Some(delay) = self.delay {
            return Utc::now() + Duration::from_std(delay).unwrap_or_default();
        }
        Utc::now()
    }
}
