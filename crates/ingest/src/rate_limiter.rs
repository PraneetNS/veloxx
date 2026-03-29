//! In-memory token-bucket rate limiter scoped per tenant.
//!
//! Each tenant gets its own token bucket.  Buckets are created on first
//! access and refilled continuously in the background.
//!
//! This implementation uses only in-memory state (no Redis dependency)
//! because rate limiting at the ingest layer needs sub-millisecond
//! decisions.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;
use tracing::warn;

/// A single tenant's token bucket state.
struct Bucket {
    tokens:        f64,
    capacity:      f64,
    refill_rate:   f64,   // tokens per second
    last_refill:   Instant,
}

impl Bucket {
    fn new(capacity: f64) -> Self {
        Self {
            tokens:      capacity,
            capacity,
            refill_rate: capacity,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time since last refill.
    fn refill(&mut self) {
        let now     = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Attempt to consume `n` tokens.  Returns `true` if allowed.
    fn try_consume(&mut self, n: f64) -> bool {
        self.refill();
        if self.tokens >= n {
            self.tokens -= n;
            true
        } else {
            false
        }
    }
}

/// Shared rate limiter holding buckets for all tenants.
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<Mutex<HashMap<Uuid, Bucket>>>,
    default_rps: f64,
}

impl RateLimiter {
    /// Create a new limiter with a default capacity of `default_rps` events/s.
    pub fn new(default_rps: u32) -> Self {
        Self {
            buckets: Arc::new(Mutex::new(HashMap::new())),
            default_rps: default_rps as f64,
        }
    }

    /// Returns `true` if the tenant is allowed to ingest one more event.
    ///
    /// On first call for a tenant, a new bucket is created with the
    /// configured default capacity.
    pub fn check(&self, tenant_id: Uuid) -> bool {
        let mut map = self.buckets.lock().expect("rate_limiter mutex poisoned");
        let bucket = map
            .entry(tenant_id)
            .or_insert_with(|| Bucket::new(self.default_rps));
        let allowed = bucket.try_consume(1.0);
        if !allowed {
            warn!(tenant_id = %tenant_id, "rate limit exceeded");
        }
        allowed
    }

    /// Override the rate limit for a specific tenant.
    pub fn set_limit(&self, tenant_id: Uuid, rps: u32) {
        let mut map = self.buckets.lock().expect("rate_limiter mutex poisoned");
        let cap = rps as f64;
        map.insert(tenant_id, Bucket::new(cap));
    }

    /// Drain stale entries (tenants not seen for > `max_idle`).
    pub fn cleanup(&self, max_idle: Duration) {
        let mut map = self.buckets.lock().expect("rate_limiter mutex poisoned");
        let now = Instant::now();
        map.retain(|_, b| now.duration_since(b.last_refill) < max_idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_allow_deny() {
        let limiter = RateLimiter::new(2);
        let tid = Uuid::new_v4();
        assert!(limiter.check(tid));
        assert!(limiter.check(tid));
        // bucket exhausted
        assert!(!limiter.check(tid));
    }
}
