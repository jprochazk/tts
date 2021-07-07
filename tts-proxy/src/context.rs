use actix_web::web;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::config;

pub type CtxData = web::Data<ProxyContext>;

#[derive(Debug)]
pub struct ProxyContext {
    /// The config being used by the app.
    pub config: config::Config,
    /// The connected IPs and their rate limits.
    user_limits: governor::RateLimiter<
        String,
        governor::state::keyed::DefaultKeyedStateStore<String>,
        governor::clock::DefaultClock,
    >,
    /// The number of TTS requests currently being processed.
    requests_being_processed: AtomicUsize,
}

impl Default for ProxyContext {
    fn default() -> Self {
        Self::new(config::Config::default())
    }
}

impl ProxyContext {
    pub fn new(config: config::Config) -> Self {
        Self {
            user_limits: governor::RateLimiter::keyed(governor::Quota::per_minute(
                config.request_limit_per_minute_per_ip,
            )),
            requests_being_processed: AtomicUsize::new(0),
            config,
        }
    }

    async fn can_accommodate_request(&self, ip: String) -> bool {
        self.user_limits
            .until_key_ready_with_jitter(
                &ip,
                governor::Jitter::new(Duration::from_secs(1), Duration::from_secs(1)),
            )
            .await;

        if self.requests_being_processed.load(Ordering::SeqCst)
            >= self.config.max_concurrent_requests.get()
        {
            false
        } else {
            self.requests_being_processed.fetch_add(1, Ordering::SeqCst);
            true
        }
    }

    fn decrease_request_count(&self) {
        self.requests_being_processed.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct RequestGuard {
    ctx: CtxData,
}

impl RequestGuard {
    async fn new(ctx: CtxData, ip: String) -> Option<Self> {
        if ctx.can_accommodate_request(ip).await {
            Some(Self { ctx })
        } else {
            None
        }
    }
}

impl std::ops::Drop for RequestGuard {
    fn drop(&mut self) {
        self.ctx.decrease_request_count();
    }
}

pub trait RequestGuardExt {
    fn try_accommodate_request<'a>(
        &'a self,
        ip: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<RequestGuard>> + Send + 'a>>
    where
        Self: 'a;
}
impl RequestGuardExt for CtxData {
    fn try_accommodate_request<'a>(
        &'a self,
        ip: String,
    ) -> Pin<Box<dyn std::future::Future<Output = Option<RequestGuard>> + Send + 'a>>
    where
        Self: 'a,
    {
        async fn try_accommodate_request(_self: &CtxData, ip: String) -> Option<RequestGuard> {
            RequestGuard::new(_self.clone(), ip).await
        }

        Box::pin(try_accommodate_request(self, ip))
    }
}
