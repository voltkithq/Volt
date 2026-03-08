use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::IpcError;

/// Rate limiter using a sliding window counter.
pub struct RateLimiter {
    max_requests: u32,
    window: Duration,
    requests: VecDeque<Instant>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            requests: VecDeque::new(),
        }
    }

    /// Check if a request is allowed. Returns `Ok(())` if allowed, error if rate limited.
    pub fn check(&mut self) -> Result<(), IpcError> {
        let now = Instant::now();
        while let Some(oldest) = self.requests.front() {
            if now.duration_since(*oldest) < self.window {
                break;
            }
            self.requests.pop_front();
        }

        if self.requests.len() as u32 >= self.max_requests {
            return Err(IpcError::RateLimitExceeded(self.max_requests));
        }

        self.requests.push_back(now);
        Ok(())
    }
}
