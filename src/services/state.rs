use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    infrastructure::{
        chrono_clock::ChronoClock, configuration::ApplicationConfiguration,
        json_web_token::JsonWebToken,
    },
    services::{
        authorization::{
            persistence::{AuthorizationPersistence, InMemorySessionRepository},
            service::AuthorizationService,
        },
        errors::set_rate_limit_retry_after_ms,
        repositories::{SessionRepository, UserRepository},
        traits::{Clock, JsonWebTokenAdapter},
    },
};

#[derive(Clone)]
#[non_exhaustive]
pub struct ApplicationState {
    pub authorization_service: Arc<AuthorizationService>,
    pub session_repository: Arc<dyn SessionRepository>,
    pub home_server_name: String,
    pub allow_registration: bool,
    pub rate_limiter: Arc<RateLimiterState>,
}

pub struct RateLimiterState {
    buckets: Mutex<HashMap<String, RateLimitBucket>>,
    window: Duration,
    max_requests_per_window: u32,
}

struct RateLimitBucket {
    window_started_at: Instant,
    request_count: u32,
}

impl ApplicationState {
    /// # Panics
    ///
    /// Will panic if database url is invalid or connection fails.
    #[allow(clippy::expect_used)]
    #[must_use]
    pub fn new(application_configuration: &ApplicationConfiguration) -> Self {
        set_rate_limit_retry_after_ms(application_configuration.rate_limit.retry_after_ms);

        let user_repository: Arc<dyn UserRepository> = Arc::new(
            AuthorizationPersistence::new(&application_configuration.database.url)
                .expect("failed to initialize authorization postgres pool"),
        );
        let session_repository: Arc<dyn SessionRepository> =
            Arc::new(InMemorySessionRepository::default());
        let json_web_token_adapter: Arc<dyn JsonWebTokenAdapter> = Arc::new(JsonWebToken);
        let clock: Arc<dyn Clock> = Arc::new(ChronoClock);

        let home_server_name = format!(
            "{}:{}",
            application_configuration.server.host, application_configuration.server.port
        );

        let authorization_service = Arc::new(AuthorizationService::new(
            user_repository,
            Arc::<dyn SessionRepository>::clone(&session_repository),
            json_web_token_adapter,
            clock,
            home_server_name.clone(),
            application_configuration.authenification.clone(),
        ));
        let rate_limiter = Arc::new(RateLimiterState::new(Duration::from_mins(1), 120));

        Self {
            authorization_service,
            session_repository,
            home_server_name,
            allow_registration: application_configuration.authenification.allow_registration,
            rate_limiter,
        }
    }
}

impl RateLimiterState {
    #[must_use]
    pub fn new(window: Duration, max_requests_per_window: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            window,
            max_requests_per_window,
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    #[allow(
        clippy::significant_drop_tightening,
        reason = "drop is handled internally"
    )]
    pub fn allow_request(&self, key: &str, now: Instant) -> anyhow::Result<bool> {
        let mut buckets_guard = self
            .buckets
            .lock()
            .map_err(|_| anyhow::anyhow!("rate limiter lock failure"))?;

        let bucket = buckets_guard
            .entry(key.to_string())
            .or_insert(RateLimitBucket {
                window_started_at: now,
                request_count: 0,
            });

        if now.duration_since(bucket.window_started_at) > self.window {
            bucket.window_started_at = now;
            bucket.request_count = 0;
        }

        if bucket.request_count >= self.max_requests_per_window {
            return Ok(false);
        }

        bucket.request_count += 1;
        Ok(true)
    }
}
