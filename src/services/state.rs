use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    infrastructure::{
        chrono_clock::ChronoClock, configuration::ApplicationConfiguration,
        id_generator::AtomicIdGenerator,
    },
    services::{
        authorization::{
            persistence::{AuthorizationPersistence, InMemorySessionRepository},
            service::AuthorizationService,
        },
        messaging_events::service::MessagingEventsService,
        repositories::{RoomRepository, SessionRepository, UserRepository},
        rooms::persistence::InMemoryRoomRepository,
        rooms::service::RoomsService,
        traits::{Clock, IdGenerator},
    },
};

#[derive(Clone)]
pub struct ApplicationState {
    pub authorization_service: Arc<AuthorizationService>,
    pub rooms_service: Arc<RoomsService>,
    pub messaging_events_service: Arc<MessagingEventsService>,
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
    pub fn new(application_configuration: ApplicationConfiguration) -> Self {
        let user_repository: Arc<dyn UserRepository> =
            Arc::new(AuthorizationPersistence::default());
        let session_repository: Arc<dyn SessionRepository> =
            Arc::new(InMemorySessionRepository::default());
        let room_repository: Arc<dyn RoomRepository> = Arc::new(InMemoryRoomRepository::default());
        let id_generator: Arc<dyn IdGenerator> = Arc::new(AtomicIdGenerator::new());
        let clock: Arc<dyn Clock> = Arc::new(ChronoClock);

        let home_server_name = format!(
            "{}:{}",
            application_configuration.server.host, application_configuration.server.port
        );

        let authorization_service = Arc::new(AuthorizationService::new(
            user_repository,
            session_repository.clone(),
            id_generator.clone(),
            home_server_name.clone(),
            application_configuration.authenification.clone(),
        ));
        let rooms_service = Arc::new(RoomsService::new(
            room_repository.clone(),
            id_generator.clone(),
            home_server_name.clone(),
        ));
        let messaging_events_service = Arc::new(MessagingEventsService::new(
            room_repository,
            id_generator,
            clock,
            home_server_name.clone(),
        ));
        let rate_limiter = Arc::new(RateLimiterState::new(Duration::from_secs(60), 120));

        ApplicationState {
            authorization_service,
            rooms_service,
            messaging_events_service,
            session_repository,
            home_server_name,
            allow_registration: application_configuration.authenification.allow_registration,
            rate_limiter,
        }
    }
}

impl RateLimiterState {
    pub fn new(window: Duration, max_requests_per_window: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            window,
            max_requests_per_window,
        }
    }

    pub fn allow_request(&self, key: &str, now: Instant) -> anyhow::Result<bool> {
        let mut buckets = self
            .buckets
            .lock()
            .map_err(|_| anyhow::anyhow!("rate limiter lock failure"))?;

        let bucket = buckets.entry(key.to_owned()).or_insert(RateLimitBucket {
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
