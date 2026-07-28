use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

pub struct RateLimiter {
    max_messages: u32,
    cooldown_seconds: u64,
    buckets: Mutex<HashMap<String, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(max_messages_per_minute: u32, cooldown_seconds: u64) -> Self {
        Self {
            max_messages: max_messages_per_minute,
            cooldown_seconds,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a message can be sent. Returns true if allowed.
    /// `key` is the group_id or sender_uuid for DMs.
    pub fn check(&self, key: &str) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let now = Instant::now();

        let (last_reset, tokens) = buckets
            .entry(key.to_string())
            .or_insert_with(|| (now, self.max_messages));

        if now.duration_since(*last_reset).as_secs() >= self.cooldown_seconds {
            *last_reset = now;
            *tokens = self.max_messages;
        }

        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }
}
