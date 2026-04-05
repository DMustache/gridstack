use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, State},
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{RETRY_AFTER, USER_AGENT},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use models::error::AppResult;
use ruma::exports::serde_json::json;
use tokio::sync::Mutex;

use crate::{services::authorization::layers::try_extract_bearer_token, state::ServerState};

const CLEANUP_INTERVAL: usize = 512;
const DEFAULT_RULE: RateLimitRule = RateLimitRule::new("client", 60, 60);

#[derive(Debug)]
struct ClientWindow {
    started_at: Instant,
    request_count: u32,
    window: Duration,
}

#[derive(Debug, Clone, Copy)]
struct RateLimitRule {
    name: &'static str,
    max_requests: u32,
    window: Duration,
}

impl RateLimitRule {
    const fn new(name: &'static str, max_requests: u32, window_seconds: u64) -> Self {
        Self {
            name,
            max_requests,
            window: Duration::from_secs(window_seconds),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RateLimitDecision {
    is_allowed: bool,
    retry_after: Duration,
}

#[derive(Debug, Default)]
pub(crate) struct ClientRateLimiter {
    windows: Mutex<HashMap<String, ClientWindow>>,
    requests_seen: AtomicUsize,
}

impl ClientRateLimiter {
    async fn check(&self, key: String, rule: RateLimitRule) -> RateLimitDecision {
        let now = Instant::now();
        let mut windows = self.windows.lock().await;

        if self.requests_seen.fetch_add(1, Ordering::Relaxed) % CLEANUP_INTERVAL == 0 {
            windows.retain(|_, window| now.duration_since(window.started_at) < window.window * 2);
        }

        let window = windows.entry(key).or_insert_with(|| ClientWindow {
            started_at: now,
            request_count: 0,
            window: rule.window,
        });

        if now.duration_since(window.started_at) >= rule.window {
            window.started_at = now;
            window.request_count = 0;
            window.window = rule.window;
        }

        if window.request_count < rule.max_requests {
            window.request_count += 1;

            return RateLimitDecision {
                is_allowed: true,
                retry_after: Duration::ZERO,
            };
        }

        let retry_after = rule
            .window
            .saturating_sub(now.duration_since(window.started_at))
            .max(Duration::from_millis(1));

        RateLimitDecision {
            is_allowed: false,
            retry_after,
        }
    }
}

pub(crate) async fn rate_limit(
    State(state): State<ServerState>,
    request: axum::extract::Request,
    next: Next,
) -> AppResult<Response> {
    if !state.is_rate_limiting_enabled {
        return Ok(next.run(request).await);
    }

    let decision = state
        .rate_limiter
        .check(
            build_rate_limit_key(request.headers(), request.uri().path(), &request),
            DEFAULT_RULE,
        )
        .await;

    if decision.is_allowed {
        return Ok(next.run(request).await);
    }

    Ok(rate_limit_response(decision.retry_after))
}

fn build_rate_limit_key(
    headers: &HeaderMap,
    path: &str,
    request: &axum::extract::Request,
) -> String {
    if let Some(token) = try_extract_bearer_token(headers) {
        return format!("{}:access-token:{token}:{path}", DEFAULT_RULE.name);
    }

    if let Some(forwarded_ip) = forwarded_ip(headers) {
        return format!("{}:forwarded-ip:{forwarded_ip}:{path}", DEFAULT_RULE.name);
    }

    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return format!("{}:socket-ip:{}:{path}", DEFAULT_RULE.name, addr.ip());
    }

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown");

    format!(
        "{}:fingerprint:{}:{path}:{user_agent}",
        DEFAULT_RULE.name,
        request.method()
    )
}

fn forwarded_ip(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
        })
}

fn rate_limit_response(retry_after: Duration) -> Response {
    let retry_after_ms = retry_after.as_millis().min(u64::MAX as u128) as u64;
    let retry_after_seconds = retry_after.as_secs().max(1).to_string();
    let body = json!({
        "errcode": "M_LIMIT_EXCEEDED",
        "error": "Too Many Requests",
        "retry_after_ms": retry_after_ms,
    });

    let mut response = (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    response.headers_mut().insert(
        RETRY_AFTER,
        HeaderValue::from_str(&retry_after_seconds)
            .unwrap_or_else(|_| HeaderValue::from_static("1")),
    );

    response
}
