use std::collections::VecDeque;
use std::time::Instant;

use crate::js_runtime::{IPC_RATE_LIMIT_MAX_REQUESTS, IPC_RATE_LIMIT_WINDOW};

#[derive(Default)]
pub(super) struct IpcRateLimiterState {
    requests: VecDeque<Instant>,
}

impl IpcRateLimiterState {
    pub(super) fn check_rate_limit(&mut self) -> Result<(), String> {
        let now = Instant::now();
        while let Some(oldest) = self.requests.front() {
            if now.duration_since(*oldest) < IPC_RATE_LIMIT_WINDOW {
                break;
            }
            self.requests.pop_front();
        }

        if self.requests.len() >= IPC_RATE_LIMIT_MAX_REQUESTS {
            return Err(format!(
                "rate limit exceeded: {IPC_RATE_LIMIT_MAX_REQUESTS} requests/second"
            ));
        }

        self.requests.push_back(now);
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn fill_to_limit(&mut self) {
        self.requests.clear();
        for _ in 0..IPC_RATE_LIMIT_MAX_REQUESTS {
            self.requests.push_back(Instant::now());
        }
    }
}
