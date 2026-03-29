//! Deduplication window for alerts.
//!
//! Prevents the same alert rule from firing more than once within its
//! configured cooldown period.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

struct Entry {
    last_fired: Instant,
    cooldown: Duration,
}

/// In-memory deduplication window.
pub struct DedupWindow {
    entries: Mutex<HashMap<Uuid, Entry>>,
}

impl DedupWindow {
    pub fn new() -> Self {
        Self { entries: Mutex::new(HashMap::new()) }
    }

    /// Returns `true` if the alert for `rule_id` may fire (not in cooldown).
    /// Records the fire time if allowed.
    pub fn allow(&self, rule_id: Uuid, cooldown_secs: u64) -> bool {
        let mut map = self.entries.lock().expect("dedup mutex poisoned");
        let now = Instant::now();

        if let Some(entry) = map.get_mut(&rule_id) {
            if now.duration_since(entry.last_fired) < entry.cooldown {
                return false;
            }
            entry.last_fired = now;
            entry.cooldown   = Duration::from_secs(cooldown_secs);
        } else {
            map.insert(rule_id, Entry {
                last_fired: now,
                cooldown:   Duration::from_secs(cooldown_secs),
            });
        }

        true
    }
}
