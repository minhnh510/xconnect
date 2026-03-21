#[derive(Debug, Clone)]
pub struct PresenceBackend {
    pub redis_url: Option<String>,
}

impl PresenceBackend {
    pub fn new(redis_url: Option<String>) -> Self {
        Self { redis_url }
    }

    pub fn mode(&self) -> &'static str {
        if self.redis_url.is_some() {
            "redis-configured"
        } else {
            "in-memory"
        }
    }
}
