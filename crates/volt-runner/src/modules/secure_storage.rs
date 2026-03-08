use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const BACKEND_ENV: &str = "VOLT_SECURE_STORAGE_BACKEND";
const BACKEND_MEMORY: &str = "memory";

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecureStorageBackend {
    NativeKeyring,
    InMemoryFallback,
    UnsupportedPlatform,
}

pub trait SecureStorageAdapter: Send + Sync {
    #[cfg_attr(not(test), allow(dead_code))]
    fn backend(&self) -> SecureStorageBackend;
    fn set(&self, key: &str, value: &str) -> Result<(), String>;
    fn get(&self, key: &str) -> Result<Option<String>, String>;
    fn delete(&self, key: &str) -> Result<(), String>;

    fn has(&self, key: &str) -> Result<bool, String> {
        self.get(key).map(|value| value.is_some())
    }
}

pub fn create_secure_storage_adapter(app_name: &str) -> Arc<dyn SecureStorageAdapter> {
    create_secure_storage_adapter_with_override(
        app_name,
        std::env::var(BACKEND_ENV).ok().as_deref(),
    )
}

/// Each call creates an independent storage instance. For shared state,
/// reuse the singleton from `secure_storage_adapter()`. Creating multiple
/// instances with the "memory" backend results in isolated storage.
pub fn create_secure_storage_adapter_with_override(
    app_name: &str,
    backend_override: Option<&str>,
) -> Arc<dyn SecureStorageAdapter> {
    if should_use_in_memory_backend(backend_override) {
        return Arc::new(InMemoryAdapter::default());
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        return Arc::new(KeyringAdapter::new(build_service_name(app_name)));
    }

    #[allow(unreachable_code)]
    Arc::new(UnsupportedPlatformAdapter)
}

fn should_use_in_memory_backend(backend_override: Option<&str>) -> bool {
    backend_override
        .map(|value| value.trim().eq_ignore_ascii_case(BACKEND_MEMORY))
        .unwrap_or(false)
}

fn build_service_name(app_name: &str) -> String {
    let sanitized = app_name
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    let compact = sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let suffix = if compact.is_empty() {
        "app".to_string()
    } else {
        compact
    };

    format!("volt.{suffix}.secure-storage")
}

struct KeyringAdapter {
    service_name: String,
}

impl KeyringAdapter {
    fn new(service_name: String) -> Self {
        Self { service_name }
    }

    fn entry(&self, key: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service_name, key)
            .map_err(|error| format!("failed to create secure storage entry: {error}"))
    }
}

impl SecureStorageAdapter for KeyringAdapter {
    fn backend(&self) -> SecureStorageBackend {
        SecureStorageBackend::NativeKeyring
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let entry = self.entry(key)?;
        entry
            .set_password(value)
            .map_err(|error| format!("failed to store secure value: {error}"))
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let entry = self.entry(key)?;
        match entry.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(format!("failed to read secure value: {error}")),
        }
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let entry = self.entry(key)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(format!("failed to delete secure value: {error}")),
        }
    }
}

#[derive(Default)]
struct InMemoryAdapter {
    values: Mutex<HashMap<String, String>>,
}

impl SecureStorageAdapter for InMemoryAdapter {
    fn backend(&self) -> SecureStorageBackend {
        SecureStorageBackend::InMemoryFallback
    }

    fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|error| format!("secure storage lock poisoned: {error}"))?;
        values.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<Option<String>, String> {
        let values = self
            .values
            .lock()
            .map_err(|error| format!("secure storage lock poisoned: {error}"))?;
        Ok(values.get(key).cloned())
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let mut values = self
            .values
            .lock()
            .map_err(|error| format!("secure storage lock poisoned: {error}"))?;
        values.remove(key);
        Ok(())
    }
}

struct UnsupportedPlatformAdapter;

impl SecureStorageAdapter for UnsupportedPlatformAdapter {
    fn backend(&self) -> SecureStorageBackend {
        SecureStorageBackend::UnsupportedPlatform
    }

    fn set(&self, _key: &str, _value: &str) -> Result<(), String> {
        Err("secure storage is not supported on this platform".to_string())
    }

    fn get(&self, _key: &str) -> Result<Option<String>, String> {
        Err("secure storage is not supported on this platform".to_string())
    }

    fn delete(&self, _key: &str) -> Result<(), String> {
        Err("secure storage is not supported on this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn service_name_sanitizes_input() {
        assert_eq!(
            build_service_name("Volt IPC Demo"),
            "volt.volt-ipc-demo.secure-storage"
        );
        assert_eq!(build_service_name(""), "volt.app.secure-storage");
    }

    #[test]
    fn in_memory_adapter_supports_crud() {
        let adapter = InMemoryAdapter::default();
        adapter.set("token", "secret").expect("set value");
        assert_eq!(
            adapter.get("token").expect("get value"),
            Some("secret".to_string())
        );
        assert!(adapter.has("token").expect("has value"));
        adapter.delete("token").expect("delete value");
        assert_eq!(adapter.get("token").expect("after delete"), None);
    }

    #[test]
    fn in_memory_adapter_handles_missing_keys() {
        let adapter = InMemoryAdapter::default();
        assert_eq!(adapter.get("missing").expect("missing key"), None);
        assert!(!adapter.has("missing").expect("missing key presence"));
        adapter.delete("missing").expect("delete missing key");
    }

    #[test]
    fn adapter_selection_supports_memory_override() {
        let adapter = create_secure_storage_adapter_with_override("Volt", Some(BACKEND_MEMORY));
        assert_eq!(adapter.backend(), SecureStorageBackend::InMemoryFallback);
    }

    #[test]
    fn adapter_selection_uses_native_on_supported_platforms() {
        let adapter = create_secure_storage_adapter_with_override("Volt", None);
        #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
        assert_eq!(adapter.backend(), SecureStorageBackend::NativeKeyring);
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        assert_eq!(adapter.backend(), SecureStorageBackend::UnsupportedPlatform);
    }

    #[test]
    fn in_memory_override_isolation_is_ci_stable() {
        let first = create_secure_storage_adapter_with_override("Volt", Some(BACKEND_MEMORY));
        let second = create_secure_storage_adapter_with_override("Volt", Some(BACKEND_MEMORY));

        first.set("token", "first").expect("set on first adapter");
        assert_eq!(
            first.get("token").expect("get first token"),
            Some("first".to_string())
        );
        assert_eq!(second.get("token").expect("get second token"), None);
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    fn run_native_keyring_roundtrip_when_enabled(app_name: &str) {
        let enabled = std::env::var("VOLT_SECURE_STORAGE_NATIVE_TESTS")
            .map(|value| value.trim() == "1")
            .unwrap_or(false);
        if !enabled {
            return;
        }

        let adapter = create_secure_storage_adapter_with_override(app_name, None);
        if adapter.backend() != SecureStorageBackend::NativeKeyring {
            return;
        }

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before unix epoch")
            .as_nanos();
        let key = format!("native-secure-storage-{nonce}");
        let value = format!("secret-{nonce}");

        let _ = adapter.delete(&key);
        adapter.set(&key, &value).expect("set native keyring value");
        assert_eq!(
            adapter.get(&key).expect("get native keyring value"),
            Some(value.clone())
        );
        assert!(adapter.has(&key).expect("has native keyring value"));
        adapter.delete(&key).expect("delete native keyring value");
        assert_eq!(adapter.get(&key).expect("after delete"), None);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn native_keyring_roundtrip_windows() {
        run_native_keyring_roundtrip_when_enabled("Volt Native Keyring Test Windows");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_keyring_roundtrip_macos() {
        run_native_keyring_roundtrip_when_enabled("Volt Native Keyring Test macOS");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn native_keyring_roundtrip_linux() {
        run_native_keyring_roundtrip_when_enabled("Volt Native Keyring Test Linux");
    }
}
