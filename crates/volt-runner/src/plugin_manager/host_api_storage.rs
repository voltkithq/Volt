mod store;

use std::path::PathBuf;

use serde_json::Value;

use super::{PLUGIN_STORAGE_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::lock_error;

const STORAGE_DIR: &str = "storage";

impl PluginManager {
    pub(super) fn handle_storage_request(
        &self,
        plugin_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        // Plugin storage requests are serialized per plugin today:
        // the host reads one plugin message at a time and the plugin-side
        // request API waits synchronously for the reply before sending the
        // next storage operation. If that transport model changes, this code
        // will need an explicit per-plugin storage lock.
        let (storage_root, should_reconcile) = self.prepare_storage_root(plugin_id)?;
        let mut storage =
            store::PluginStorage::open(&storage_root, should_reconcile).map_err(storage_error)?;

        match operation {
            "get" => Ok(storage
                .get(store::required_key(payload)?)?
                .map(Value::String)
                .unwrap_or(Value::Null)),
            "set" => {
                storage.set(
                    store::required_key(payload)?,
                    store::required_value(payload)?,
                )?;
                Ok(Value::Null)
            }
            "has" => Ok(Value::Bool(storage.has(store::required_key(payload)?)?)),
            "delete" => {
                storage.delete(store::required_key(payload)?)?;
                Ok(Value::Null)
            }
            "keys" => Ok(serde_json::json!(storage.keys())),
            _ => Err(PluginRuntimeError {
                code: PLUGIN_STORAGE_ERROR_CODE.to_string(),
                message: format!("unsupported storage operation '{operation}'"),
            }),
        }
    }

    fn prepare_storage_root(&self, plugin_id: &str) -> Result<(PathBuf, bool), PluginRuntimeError> {
        let data_root = self.plugin_data_root(plugin_id)?;
        volt_core::fs::mkdir(&data_root, STORAGE_DIR)
            .map_err(|error| storage_error(error.to_string()))?;
        let storage_root = volt_core::fs::safe_resolve(&data_root, STORAGE_DIR)
            .map_err(|error| storage_error(error.to_string()))?;

        let mut registry = self.inner.registry.lock().map_err(lock_error)?;
        let Some(record) = registry.plugins.get_mut(plugin_id) else {
            return Err(PluginRuntimeError {
                code: PLUGIN_STORAGE_ERROR_CODE.to_string(),
                message: format!("plugin '{plugin_id}' is not available"),
            });
        };
        let should_reconcile = !record.storage_reconciled;
        record.storage_reconciled = true;
        Ok((storage_root, should_reconcile))
    }
}

fn storage_error(message: impl Into<String>) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_STORAGE_ERROR_CODE.to_string(),
        message: message.into(),
    }
}
