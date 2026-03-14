//! Scoped file watcher module.
//!
//! Wraps the `notify` crate to provide debounced, scope-relative file system
//! events. Events are emitted on a crossbeam channel that callers poll.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

static WATCHER_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A file system event scoped to the watched directory.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatchEvent {
    /// Event kind: "create", "change", "delete", "rename", "overflow"
    pub kind: String,
    /// Scope-relative path of the affected file/directory.
    pub path: String,
    /// For rename events, the old scope-relative path (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    /// Whether the event target is a directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dir: Option<bool>,
}

struct WatcherEntry {
    _watcher: RecommendedWatcher,
    receiver: crossbeam_channel::Receiver<WatchEvent>,
    #[allow(dead_code)]
    root: PathBuf,
}

static WATCHERS: Mutex<Option<HashMap<String, WatcherEntry>>> = Mutex::new(None);

fn with_watchers<F, R>(f: F) -> R
where
    F: FnOnce(&mut HashMap<String, WatcherEntry>) -> R,
{
    let mut guard = WATCHERS.lock().unwrap_or_else(|e| e.into_inner());
    let store = guard.get_or_insert_with(HashMap::new);
    f(store)
}

fn classify_event(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Create(_) => "create",
        EventKind::Modify(notify::event::ModifyKind::Name(_)) => "rename",
        EventKind::Modify(_) => "change",
        EventKind::Remove(_) => "delete",
        EventKind::Other => "change",
        EventKind::Any => "change",
        EventKind::Access(_) => "change",
    }
}

fn make_relative(root: &Path, absolute: &Path) -> Option<String> {
    absolute
        .strip_prefix(root)
        .ok()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
}

/// Start watching a directory. Returns a watcher ID.
pub fn start_watch(root: PathBuf, recursive: bool, debounce_ms: u64) -> Result<String, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("cannot watch: {e}"))?;

    if !canonical_root.is_dir() {
        return Err("watch target must be a directory".to_string());
    }

    let (tx, rx) = crossbeam_channel::bounded::<WatchEvent>(4096);
    let root_for_handler = canonical_root.clone();

    let debounce_duration = Duration::from_millis(debounce_ms.max(50));

    let config = Config::default().with_poll_interval(debounce_duration);

    let mut watcher = RecommendedWatcher::new(
        move |result: Result<Event, notify::Error>| match result {
            Ok(event) => {
                let kind = classify_event(&event.kind);
                for path in &event.paths {
                    if let Some(rel) = make_relative(&root_for_handler, path) {
                        let _ = tx.send(WatchEvent {
                            kind: kind.to_string(),
                            path: rel,
                            old_path: None,
                            is_dir: path.is_dir().then_some(true),
                        });
                    }
                }
            }
            Err(_) => {
                let _ = tx.send(WatchEvent {
                    kind: "overflow".to_string(),
                    path: String::new(),
                    old_path: None,
                    is_dir: None,
                });
            }
        },
        config,
    )
    .map_err(|e| format!("failed to create watcher: {e}"))?;

    let mode = if recursive {
        RecursiveMode::Recursive
    } else {
        RecursiveMode::NonRecursive
    };

    watcher
        .watch(&canonical_root, mode)
        .map_err(|e| format!("failed to start watching: {e}"))?;

    let id = format!(
        "watcher_{:x}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        WATCHER_COUNTER.fetch_add(1, Ordering::Relaxed)
    );

    with_watchers(|watchers| {
        watchers.insert(
            id.clone(),
            WatcherEntry {
                _watcher: watcher,
                receiver: rx,
                root: canonical_root,
            },
        );
    });

    Ok(id)
}

/// Drain all pending events from a watcher.
pub fn drain_events(watcher_id: &str) -> Result<Vec<WatchEvent>, String> {
    with_watchers(|watchers| {
        let entry = watchers
            .get(watcher_id)
            .ok_or_else(|| "watcher not found".to_string())?;

        let mut events = Vec::new();
        while let Ok(event) = entry.receiver.try_recv() {
            events.push(event);
        }
        Ok(events)
    })
}

/// Stop and remove a watcher.
pub fn stop_watch(watcher_id: &str) -> Result<(), String> {
    with_watchers(|watchers| {
        watchers
            .remove(watcher_id)
            .map(|_| ())
            .ok_or_else(|| "watcher not found".to_string())
    })
}

/// Get the root path of an active watcher.
#[allow(dead_code)]
pub fn watcher_root(watcher_id: &str) -> Result<PathBuf, String> {
    with_watchers(|watchers| {
        watchers
            .get(watcher_id)
            .map(|entry| entry.root.clone())
            .ok_or_else(|| "watcher not found".to_string())
    })
}

/// Get count of active watchers.
#[allow(dead_code)]
pub fn watcher_count() -> usize {
    with_watchers(|watchers| watchers.len())
}

/// Stop all watchers.
#[allow(dead_code)]
pub fn clear_watchers() {
    with_watchers(|watchers| watchers.clear());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_start_and_stop_watch() {
        let dir = env::temp_dir().join("volt_test_watcher_start_stop");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let id = start_watch(dir.clone(), true, 100).unwrap();
        assert!(id.starts_with("watcher_"));

        stop_watch(&id).unwrap();

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_stop_nonexistent_watcher() {
        let result = stop_watch("nonexistent_watcher");
        assert!(result.is_err());
    }

    #[test]
    fn test_watch_nonexistent_dir() {
        let result = start_watch(
            PathBuf::from("/definitely/does/not/exist/volt_test"),
            true,
            100,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_watch_detects_file_create() {
        let dir = env::temp_dir().join("volt_test_watcher_create");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let id = start_watch(dir.clone(), true, 50).unwrap();

        // Create a file
        fs::write(dir.join("new_file.txt"), b"hello").unwrap();

        // Give the watcher time to process
        std::thread::sleep(Duration::from_millis(500));

        let events = drain_events(&id).unwrap();
        // We should have at least one event (create or change)
        // The exact events depend on the platform watcher implementation
        assert!(
            !events.is_empty(),
            "expected at least one event after file creation"
        );

        stop_watch(&id).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_drain_events_empty() {
        let dir = env::temp_dir().join("volt_test_watcher_drain_empty");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let id = start_watch(dir.clone(), true, 100).unwrap();

        // Drain any initial events the watcher may emit on startup
        std::thread::sleep(Duration::from_millis(200));
        let _ = drain_events(&id).unwrap();

        // Now a second drain should be empty (no new filesystem changes)
        let events = drain_events(&id).unwrap();
        assert!(events.is_empty());

        stop_watch(&id).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_make_relative() {
        let root = PathBuf::from("/base/dir");
        assert_eq!(
            make_relative(&root, &PathBuf::from("/base/dir/sub/file.txt")),
            Some("sub/file.txt".to_string())
        );
        assert_eq!(
            make_relative(&root, &PathBuf::from("/other/dir/file.txt")),
            None
        );
    }

    #[test]
    fn test_classify_event_kinds() {
        assert_eq!(
            classify_event(&EventKind::Create(notify::event::CreateKind::File)),
            "create"
        );
        assert_eq!(
            classify_event(&EventKind::Remove(notify::event::RemoveKind::File)),
            "delete"
        );
        assert_eq!(
            classify_event(&EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content
            ))),
            "change"
        );
        assert_eq!(
            classify_event(&EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::Both
            ))),
            "rename"
        );
    }
}
