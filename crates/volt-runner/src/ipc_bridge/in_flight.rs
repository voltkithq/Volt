use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(super) struct InFlightTracker {
    max_per_window: usize,
    max_total: usize,
    counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl InFlightTracker {
    pub(super) fn new(max_per_window: usize, max_total: usize) -> Self {
        Self {
            max_per_window,
            max_total,
            counts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(super) fn try_acquire(&self, js_window_id: &str) -> bool {
        let mut guard = match self.counts.lock() {
            Ok(guard) => guard,
            Err(_) => return false,
        };

        let in_flight = guard.get(js_window_id).copied().unwrap_or(0);
        let total_in_flight: usize = guard.values().sum();
        if total_in_flight >= self.max_total || in_flight >= self.max_per_window {
            return false;
        }

        guard.insert(js_window_id.to_string(), in_flight + 1);
        true
    }

    pub(super) fn release(&self, js_window_id: &str) {
        let mut guard = match self.counts.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        match guard.get(js_window_id).copied().unwrap_or(0) {
            0 | 1 => {
                guard.remove(js_window_id);
            }
            count => {
                guard.insert(js_window_id.to_string(), count - 1);
            }
        }
    }

    pub(super) fn max_per_window(&self) -> usize {
        self.max_per_window
    }

    pub(super) fn max_total(&self) -> usize {
        self.max_total
    }

    #[cfg(test)]
    pub(super) fn in_flight_for(&self, js_window_id: &str) -> usize {
        self.counts
            .lock()
            .expect("in-flight map")
            .get(js_window_id)
            .copied()
            .unwrap_or(0)
    }
}
