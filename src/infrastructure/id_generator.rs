use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::services::traits::IdGenerator;

#[derive(Default)]
pub struct AtomicIdGenerator {
    token_sequence: AtomicU64,
    room_sequence: AtomicU64,
    event_sequence: AtomicU64,
}

impl AtomicIdGenerator {
    pub fn new() -> Self {
        Self {
            token_sequence: AtomicU64::new(1),
            room_sequence: AtomicU64::new(1),
            event_sequence: AtomicU64::new(1),
        }
    }
}

impl IdGenerator for AtomicIdGenerator {
    fn next_access_token(&self, user_identifier: &str) -> String {
        let sequence = self.token_sequence.fetch_add(1, Ordering::Relaxed);
        format!(
            "access_{}_{}_{}",
            user_identifier.replace('@', "").replace(':', "_"),
            current_unix_milliseconds(),
            sequence
        )
    }

    fn next_room_identifier(&self, home_server_name: &str) -> String {
        let sequence = self.room_sequence.fetch_add(1, Ordering::Relaxed);
        format!(
            "!room{}_{}:{}",
            sequence,
            current_unix_milliseconds(),
            home_server_name
        )
    }

    fn next_event_identifier(&self, home_server_name: &str) -> String {
        let sequence = self.event_sequence.fetch_add(1, Ordering::Relaxed);
        format!(
            "$event{}_{}:{}",
            sequence,
            current_unix_milliseconds(),
            home_server_name
        )
    }
}

pub fn current_unix_milliseconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}
