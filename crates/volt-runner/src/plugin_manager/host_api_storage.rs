use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{PLUGIN_STORAGE_ERROR_CODE, PluginManager, PluginRuntimeError};
use crate::plugin_manager::host_api_helpers::lock_error;

const STORAGE_DIR: &str = "storage";
const STORAGE_INDEX_FILE: &str = "_index.json";
const STORAGE_MAX_KEY_BYTES: usize = 256;
const STORAGE_MAX_VALUE_BYTES: usize = 1024 * 1024;
const STORAGE_MAX_TOTAL_BYTES: usize = 50 * 1024 * 1024;

impl PluginManager {
    pub(super) fn handle_storage_request(
        &self,
        plugin_id: &str,
        operation: &str,
        payload: &Value,
    ) -> Result<Value, PluginRuntimeError> {
        let (storage_root, should_reconcile) = self.prepare_storage_root(plugin_id)?;
        let mut storage =
            PluginStorage::open(&storage_root, should_reconcile).map_err(storage_error)?;

        match operation {
            "get" => Ok(storage
                .get(required_key(payload)?)?
                .map(Value::String)
                .unwrap_or(Value::Null)),
            "set" => {
                storage.set(required_key(payload)?, required_value(payload)?)?;
                Ok(Value::Null)
            }
            "has" => Ok(Value::Bool(storage.has(required_key(payload)?)?)),
            "delete" => {
                storage.delete(required_key(payload)?)?;
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

struct PluginStorage {
    root: PathBuf,
    index: StorageIndex,
    total_bytes: usize,
}

impl PluginStorage {
    fn open(root: &Path, reconcile: bool) -> Result<Self, String> {
        let mut index = load_index(root)?;
        if reconcile {
            reconcile_index(root, &mut index)?;
        }
        let total_bytes = estimate_total_bytes(root, &index);
        Ok(Self {
            root: root.to_path_buf(),
            index,
            total_bytes,
        })
    }

    fn get(&self, key: String) -> Result<Option<String>, PluginRuntimeError> {
        let Some(hash) = self.index.entries.get(&key) else {
            return Ok(None);
        };
        let path = value_path(hash);
        match volt_core::fs::read_file_text(&self.root, &path) {
            Ok(value) => Ok(Some(value)),
            Err(volt_core::fs::FsError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                Ok(None)
            }
            Err(error) => Err(storage_error(error.to_string())),
        }
    }

    fn set(&mut self, key: String, value: String) -> Result<(), PluginRuntimeError> {
        let new_size = value.len();
        // Subtract old value size if key already exists.
        let old_size = self
            .index
            .entries
            .get(&key)
            .and_then(|hash| {
                let path = value_path(hash);
                volt_core::fs::stat(&self.root, &path).ok().map(|info| info.size as usize)
            })
            .unwrap_or(0);
        let projected = self.total_bytes + new_size - old_size;
        if projected > STORAGE_MAX_TOTAL_BYTES {
            return Err(storage_error(format!(
                "plugin storage quota exceeded ({projected} > {STORAGE_MAX_TOTAL_BYTES} bytes)"
            )));
        }
        let hash = hash_key(&key);
        write_bytes_atomic(&self.root, &value_path(&hash), value.as_bytes())
            .map_err(storage_error)?;
        self.index.entries.insert(key, hash);
        self.total_bytes = projected;
        save_index(&self.root, &self.index).map_err(storage_error)
    }

    fn has(&self, key: String) -> Result<bool, PluginRuntimeError> {
        Ok(self.get(key)?.is_some())
    }

    fn delete(&mut self, key: String) -> Result<(), PluginRuntimeError> {
        let Some(hash) = self.index.entries.remove(&key) else {
            return Ok(());
        };
        let value_path = value_path(&hash);
        if volt_core::fs::exists(&self.root, &value_path)
            .map_err(|error| storage_error(error.to_string()))?
        {
            volt_core::fs::remove(&self.root, &value_path)
                .map_err(|error| storage_error(error.to_string()))?;
        }
        save_index(&self.root, &self.index).map_err(storage_error)
    }

    fn keys(&self) -> Vec<String> {
        self.index.entries.keys().cloned().collect()
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StorageIndex {
    entries: BTreeMap<String, String>,
}

fn estimate_total_bytes(root: &Path, index: &StorageIndex) -> usize {
    index
        .entries
        .values()
        .filter_map(|hash| {
            let path = value_path(hash);
            volt_core::fs::stat(root, &path).ok().map(|info| info.size as usize)
        })
        .sum()
}

fn load_index(root: &Path) -> Result<StorageIndex, String> {
    match volt_core::fs::read_file_text(root, STORAGE_INDEX_FILE) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(index) => Ok(index),
            Err(error) => {
                tracing::warn!(
                    "storage index is corrupted ({}), starting with empty index",
                    error
                );
                Ok(StorageIndex::default())
            }
        },
        Err(volt_core::fs::FsError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StorageIndex::default())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn save_index(root: &Path, index: &StorageIndex) -> Result<(), String> {
    let bytes = serde_json::to_vec(index).map_err(|error| error.to_string())?;
    write_bytes_atomic(root, STORAGE_INDEX_FILE, &bytes)
}

fn reconcile_index(root: &Path, index: &mut StorageIndex) -> Result<(), String> {
    let mut changed = false;
    index.entries.retain(|_, hash| {
        let exists = volt_core::fs::exists(root, &value_path(hash)).unwrap_or(false);
        changed |= !exists;
        exists
    });

    let expected = index
        .entries
        .values()
        .map(|hash| value_path(hash))
        .collect::<std::collections::HashSet<_>>();
    for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        // Clean up orphaned .val files not referenced by the index
        if name.ends_with(".val") && !expected.contains(&name) {
            volt_core::fs::remove(root, &name).map_err(|error| error.to_string())?;
            changed = true;
        }
        // Clean up leftover .tmp files from interrupted atomic writes
        if name.ends_with(".tmp") {
            let _ = volt_core::fs::remove(root, &name);
            changed = true;
        }
    }
    if changed {
        save_index(root, index)?;
    }
    Ok(())
}

fn write_bytes_atomic(root: &Path, relative_path: &str, data: &[u8]) -> Result<(), String> {
    let temp_path = temp_path(relative_path);
    let temp_resolved = volt_core::fs::safe_resolve_for_create(root, &temp_path)
        .map_err(|error| error.to_string())?;
    let final_resolved = volt_core::fs::safe_resolve_for_create(root, relative_path)
        .map_err(|error| error.to_string())?;
    std::fs::write(&temp_resolved, data).map_err(|error| error.to_string())?;
    replace_path_atomic(&temp_resolved, &final_resolved)
}

fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn value_path(hash: &str) -> String {
    format!("{hash}.val")
}

fn temp_path(relative_path: &str) -> String {
    relative_path
        .replace(".val", ".tmp")
        .replace(".json", ".tmp")
}

fn required_key(payload: &Value) -> Result<String, PluginRuntimeError> {
    let key = payload
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| storage_error("payload is missing required 'key' string"))?;
    validate_key(key)?;
    Ok(key.to_string())
}

fn required_value(payload: &Value) -> Result<String, PluginRuntimeError> {
    let value = payload
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| storage_error("payload is missing required 'value' string"))?;
    if value.len() > STORAGE_MAX_VALUE_BYTES {
        return Err(storage_error(format!(
            "storage value exceeds {} bytes",
            STORAGE_MAX_VALUE_BYTES
        )));
    }
    Ok(value.to_string())
}

fn validate_key(key: &str) -> Result<(), PluginRuntimeError> {
    if key.is_empty() {
        return Err(storage_error("storage key must not be empty"));
    }
    if key.len() > STORAGE_MAX_KEY_BYTES {
        return Err(storage_error(format!(
            "storage key exceeds {} bytes",
            STORAGE_MAX_KEY_BYTES
        )));
    }
    if key.contains("..") || key.contains('/') || key.contains('\\') {
        return Err(storage_error(
            "storage key must not contain path traversal segments",
        ));
    }
    Ok(())
}

fn storage_error(message: impl Into<String>) -> PluginRuntimeError {
    PluginRuntimeError {
        code: PLUGIN_STORAGE_ERROR_CODE.to_string(),
        message: message.into(),
    }
}

#[cfg(windows)]
fn replace_path_atomic(from: &Path, to: &Path) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(
            lp_existing_file_name: *const u16,
            lp_new_file_name: *const u16,
            dw_flags: u32,
        ) -> i32;
    }

    let wide = |path: &Path| -> Vec<u16> {
        OsStr::new(path.as_os_str())
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let from_wide = wide(from);
    let to_wide = wide(to);
    let status = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if status == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(not(windows))]
fn replace_path_atomic(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|error| error.to_string())
}
