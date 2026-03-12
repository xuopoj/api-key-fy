use std::sync::Arc;
use chrono::Utc;
use dashmap::DashMap;
use uuid::Uuid;

#[derive(Debug)]
struct Counter {
    count: u32,
    window_start: i64, // unix timestamp (seconds)
}

#[derive(Clone)]
pub struct RateLimiter {
    counters: Arc<DashMap<Uuid, Counter>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(DashMap::new()),
        }
    }

    /// Returns true if the request is allowed, false if rate limit exceeded.
    pub fn check(&self, key_id: Uuid, limit_rpm: u32) -> bool {
        let now = Utc::now().timestamp();
        let window_start = now - (now % 60); // current minute boundary

        let mut entry = self.counters.entry(key_id).or_insert(Counter {
            count: 0,
            window_start,
        });

        if entry.window_start != window_start {
            entry.count = 0;
            entry.window_start = window_start;
        }

        if entry.count >= limit_rpm {
            return false;
        }

        entry.count += 1;
        true
    }
}
