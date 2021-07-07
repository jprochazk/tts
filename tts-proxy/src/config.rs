use std::num::{NonZeroU32, NonZeroUsize};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Config {
    pub api_url: String,
    pub request_limit_per_minute_per_ip: NonZeroU32,
    pub max_concurrent_requests: NonZeroUsize,
    pub api_timeout_seconds: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_url: crate::API_URL.into(),
            request_limit_per_minute_per_ip: NonZeroU32::new(5).unwrap(),
            max_concurrent_requests: NonZeroUsize::new(30).unwrap(),
            api_timeout_seconds: 180,
        }
    }
}
