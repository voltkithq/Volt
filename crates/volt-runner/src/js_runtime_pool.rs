use std::sync::{Arc, Mutex};

use crate::js_runtime::{JsRuntimeClient, JsRuntimeManager, JsRuntimeOptions};

mod client;
mod rate_limiter;
#[cfg(test)]
mod tests;

use rate_limiter::IpcRateLimiterState;

const MIN_POOL_SIZE: usize = 2;
const MAX_POOL_SIZE: usize = 4;

#[derive(Clone)]
pub struct JsRuntimePoolClient {
    clients: Arc<Vec<JsRuntimeClient>>,
    ipc_rate_limiter: Arc<Mutex<IpcRateLimiterState>>,
}

pub struct JsRuntimePool {
    _managers: Vec<JsRuntimeManager>,
    client: JsRuntimePoolClient,
}

impl JsRuntimePool {
    pub fn default_pool_size() -> usize {
        normalize_pool_size(num_cpus::get())
    }

    pub fn start_with_options(pool_size: usize, options: JsRuntimeOptions) -> Result<Self, String> {
        let normalized_pool_size = normalize_pool_size(pool_size);
        let mut managers = Vec::with_capacity(normalized_pool_size);
        let mut clients = Vec::with_capacity(normalized_pool_size);
        let ipc_rate_limiter = Arc::new(Mutex::new(IpcRateLimiterState::default()));

        for runtime_index in 0..normalized_pool_size {
            let manager =
                JsRuntimeManager::start_with_options(options.clone()).map_err(|error| {
                    format!(
                        "failed to start js runtime {} of {}: {error}",
                        runtime_index + 1,
                        normalized_pool_size
                    )
                })?;
            clients.push(manager.client());
            managers.push(manager);
        }

        let client = JsRuntimePoolClient {
            clients: Arc::new(clients),
            ipc_rate_limiter,
        };

        Ok(Self {
            _managers: managers,
            client,
        })
    }

    pub fn client(&self) -> JsRuntimePoolClient {
        self.client.clone()
    }

    #[cfg(test)]
    fn runtime_count(&self) -> usize {
        self._managers.len()
    }
}

fn normalize_pool_size(pool_size: usize) -> usize {
    pool_size.clamp(MIN_POOL_SIZE, MAX_POOL_SIZE)
}
