use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::plugin_manager::PluginRuntimeError;

use super::storage_error;

const STORAGE_INDEX_FILE: &str = "_index.json";
const STORAGE_MAX_KEY_BYTES: usize = 256;
const STORAGE_MAX_VALUE_BYTES: usize = 1024 * 1024;
const STORAGE_MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;

pub(super) struct PluginStorage {
    root: PathBuf,
    index: StorageIndex,
}

impl PluginStorage {
    pub(super) fn open(root: &Path, reconcile: bool) -> Result<Self, String> {
        let mut index = load_index(root)?;
        if reconcile {
            reconcile_index(root, &mut index)?;
        }
        Ok(Self {
            root: root.to_path_buf(),
            index,
        })
    }

    pub(super) fn get(&self, key: String) -> Result<Option<String>, PluginRuntimeError> {
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

    pub(super) fn set(&mut self, key: String, value: String) -> Result<(), PluginRuntimeError> {
        self.ensure_within_quota(&key, value.len() as u64)?;
        let hash = hash_key(&key);
        write_bytes_atomic(&self.root, &value_path(&hash), value.as_bytes())
            .map_err(storage_error)?;
        self.index.entries.insert(key, hash);
        save_index(&self.root, &self.index).map_err(storage_error)
    }

    pub(super) fn has(&self, key: String) -> Result<bool, PluginRuntimeError> {
        Ok(self.get(key)?.is_some())
    }

    pub(super) fn delete(&mut self, key: String) -> Result<(), PluginRuntimeError> {
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

    pub(super) fn keys(&self) -> Vec<String> {
        self.index.entries.keys().cloned().collect()
    }

    fn ensure_within_quota(
        &self,
        key: &str,
        next_value_bytes: u64,
    ) -> Result<(), PluginRuntimeError> {
        let current_total = self.total_value_bytes()?;
        let replaced_bytes = self.value_bytes_for_key(key)?;
        let projected_total = current_total
            .saturating_sub(replaced_bytes)
            .saturating_add(next_value_bytes);
        if projected_total > STORAGE_MAX_TOTAL_BYTES {
            return Err(storage_error(format!(
                "storage quota exceeded ({} bytes > {} bytes)",
                projected_total, STORAGE_MAX_TOTAL_BYTES
            )));
        }
        Ok(())
    }

    fn total_value_bytes(&self) -> Result<u64, PluginRuntimeError> {
        let mut total = 0_u64;
        for hash in self.index.entries.values() {
            let path = self.root.join(value_path(hash));
            match std::fs::metadata(&path) {
                Ok(metadata) => total = total.saturating_add(metadata.len()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(storage_error(error.to_string())),
            }
        }
        Ok(total)
    }

    fn value_bytes_for_key(&self, key: &str) -> Result<u64, PluginRuntimeError> {
        let Some(hash) = self.index.entries.get(key) else {
            return Ok(0);
        };
        let path = self.root.join(value_path(hash));
        match std::fs::metadata(path) {
            Ok(metadata) => Ok(metadata.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(error) => Err(storage_error(error.to_string())),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct StorageIndex {
    entries: BTreeMap<String, String>,
}

fn load_index(root: &Path) -> Result<StorageIndex, String> {
    match volt_core::fs::read_file_text(root, STORAGE_INDEX_FILE) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(index) => Ok(index),
            Err(error) => {
                tracing::warn!(
                    storage_root = %root.display(),
                    "plugin storage index is corrupted; rebuilding from an empty index: {error}"
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
        .collect::<HashSet<_>>();
    for entry in std::fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if (name.ends_with(".val") && !expected.contains(&name)) || name.ends_with(".tmp") {
            volt_core::fs::remove(root, &name).map_err(|error| error.to_string())?;
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
    volt_core::fs::write_file(root, &temp_path, data).map_err(|error| error.to_string())?;
    if let Err(error) = volt_core::fs::replace_file(root, &temp_path, relative_path) {
        let _ = volt_core::fs::remove(root, &temp_path);
        return Err(error.to_string());
    }
    Ok(())
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

pub(super) fn required_key(payload: &Value) -> Result<String, PluginRuntimeError> {
    let key = payload
        .get("key")
        .and_then(Value::as_str)
        .ok_or_else(|| storage_error("payload is missing required 'key' string"))?;
    validate_key(key)?;
    Ok(key.to_string())
}

pub(super) fn required_value(payload: &Value) -> Result<String, PluginRuntimeError> {
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
