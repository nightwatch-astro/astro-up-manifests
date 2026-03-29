use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks per-provider rate limit pause windows.
#[derive(Default)]
pub struct RateLimiter {
    paused_until: HashMap<String, Instant>,
}

impl RateLimiter {
    /// Check if a provider is currently paused due to rate limiting.
    pub fn is_paused(&self, provider: &str) -> bool {
        self.paused_until
            .get(provider)
            .map_or(false, |until| Instant::now() < *until)
    }

    /// Pause a provider for a given duration (from retry-after or backoff).
    pub fn pause(&mut self, provider: &str, duration: Duration) {
        let until = Instant::now() + duration;
        tracing::warn!("rate limited: pausing {provider} for {}s", duration.as_secs());
        self.paused_until.insert(provider.to_string(), until);
    }

    /// Record a rate limit response. Extracts pause duration from headers.
    pub fn record_rate_limit(&mut self, provider: &str, retry_after: Option<&str>) {
        let duration = retry_after
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        self.pause(provider, duration);
    }

    /// Get remaining pause time for a provider, if paused.
    pub fn remaining(&self, provider: &str) -> Option<Duration> {
        self.paused_until.get(provider).and_then(|until| {
            let now = Instant::now();
            if now < *until {
                Some(*until - now)
            } else {
                None
            }
        })
    }
}
