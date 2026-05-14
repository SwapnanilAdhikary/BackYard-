#[derive(Debug, Clone)]
pub struct SqliteConfig {
    pub database_url: String,
    pub poll_interval: std::time::Duration,
    pub max_connections: u32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            database_url: "sqlite://backyard.db".into(),
            poll_interval: std::time::Duration::from_millis(250),
            max_connections: 5,
        }
    }
}
