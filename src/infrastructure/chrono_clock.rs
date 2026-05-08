use chrono::Utc;

use crate::services::traits::Clock;

#[derive(Default)]
pub struct ChronoClock;

impl Clock for ChronoClock {
    fn now_unix_milliseconds(&self) -> i64 {
        Utc::now().timestamp_millis()
    }

    fn as_seconds(&self) -> i64 {
        Utc::now().timestamp()
    }
}
