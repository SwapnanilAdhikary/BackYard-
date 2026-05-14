#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub redis_url: String,
    pub pool_size: usize,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://127.0.0.1:6379".into(),
            pool_size: 10,
        }
    }
}
