use crate::app::AppEvent;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak, mpsc};
use std::time::{Duration, Instant};
use tao::event_loop::EventLoopProxy;

use super::{AppCommand, CommandBridgeError, CommandEnvelope, CommandObservabilitySnapshot};

struct BridgeInner {
    proxy: EventLoopProxy<AppEvent>,
    sender: mpsc::Sender<CommandEnvelope>,
    active: AtomicBool,
}

impl BridgeInner {
    fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }
}

/// Lifecycle guard for the global bridge context.
/// When dropped, the bridge is deactivated and detached from the global slot.
#[derive(Clone)]
pub struct BridgeLifecycle {
    inner: Arc<BridgeInner>,
}

impl BridgeLifecycle {
    /// Explicitly stop the active bridge context.
    pub fn shutdown(&self) {
        deactivate_and_clear(&self.inner);
    }
}

impl Drop for BridgeLifecycle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Registration result returned by bridge initialization.
pub struct BridgeRegistration {
    pub receiver: mpsc::Receiver<CommandEnvelope>,
    pub lifecycle: BridgeLifecycle,
}

static NEXT_COMMAND_TRACE_ID: AtomicU64 = AtomicU64::new(1);
static COMMANDS_SENT: AtomicU64 = AtomicU64::new(0);
static COMMANDS_PROCESSED: AtomicU64 = AtomicU64::new(0);
static COMMANDS_FAILED: AtomicU64 = AtomicU64::new(0);

/// Global holder for the current bridge context.
/// Uses Weak + lifecycle guard, rather than mutable global ownership.
static BRIDGE: OnceLock<Mutex<Weak<BridgeInner>>> = OnceLock::new();

fn bridge_slot() -> &'static Mutex<Weak<BridgeInner>> {
    BRIDGE.get_or_init(|| Mutex::new(Weak::new()))
}

fn store_active_bridge(inner: &Arc<BridgeInner>) -> Result<(), CommandBridgeError> {
    let mut guard = bridge_slot()
        .lock()
        .map_err(|error| CommandBridgeError::BridgeLockPoisoned(error.to_string()))?;

    if let Some(existing) = guard.upgrade()
        && existing.is_active()
    {
        return Err(CommandBridgeError::BridgeAlreadyInitialized);
    }

    *guard = Arc::downgrade(inner);
    Ok(())
}

fn load_active_bridge() -> Result<Arc<BridgeInner>, CommandBridgeError> {
    let guard = bridge_slot()
        .lock()
        .map_err(|error| CommandBridgeError::BridgeLockPoisoned(error.to_string()))?;
    let bridge = guard
        .upgrade()
        .ok_or(CommandBridgeError::EventLoopNotRunning)?;
    if !bridge.is_active() {
        return Err(CommandBridgeError::EventLoopNotRunning);
    }
    Ok(bridge)
}

fn clear_bridge_if_matches(inner: &Arc<BridgeInner>) {
    if let Ok(mut guard) = bridge_slot().lock()
        && let Some(current) = guard.upgrade()
        && Arc::ptr_eq(&current, inner)
    {
        *guard = Weak::new();
    }
}

fn deactivate_and_clear(inner: &Arc<BridgeInner>) {
    inner.deactivate();
    clear_bridge_if_matches(inner);
}

fn record_failed_command_send() {
    COMMANDS_FAILED.fetch_add(1, Ordering::Relaxed);
}

fn reset_observability_counters() {
    NEXT_COMMAND_TRACE_ID.store(1, Ordering::Relaxed);
    COMMANDS_SENT.store(0, Ordering::Relaxed);
    COMMANDS_PROCESSED.store(0, Ordering::Relaxed);
    COMMANDS_FAILED.store(0, Ordering::Relaxed);
}

/// Initialize the bridge and return command receiver + lifecycle guard.
pub fn init_bridge(
    proxy: EventLoopProxy<AppEvent>,
) -> Result<BridgeRegistration, CommandBridgeError> {
    let (tx, rx) = mpsc::channel();
    let inner = Arc::new(BridgeInner {
        proxy,
        sender: tx,
        active: AtomicBool::new(true),
    });
    store_active_bridge(&inner)?;
    reset_observability_counters();
    Ok(BridgeRegistration {
        receiver: rx,
        lifecycle: BridgeLifecycle { inner },
    })
}

/// Explicitly stop the active bridge.
pub fn shutdown_bridge() {
    if let Ok(inner) = load_active_bridge() {
        deactivate_and_clear(&inner);
    }
}

/// Backward-compatible alias for shutdown behavior.
pub fn clear_bridge() {
    shutdown_bridge();
}

/// Returns true while an app event loop bridge is active.
pub fn is_running() -> bool {
    load_active_bridge().is_ok()
}

/// Record one command that finished processing on the event loop thread.
pub fn record_processed_command() {
    COMMANDS_PROCESSED.fetch_add(1, Ordering::Relaxed);
}

/// Snapshot command-channel observability counters.
pub fn command_observability_snapshot() -> CommandObservabilitySnapshot {
    CommandObservabilitySnapshot {
        commands_sent: COMMANDS_SENT.load(Ordering::Relaxed),
        commands_processed: COMMANDS_PROCESSED.load(Ordering::Relaxed),
        commands_failed: COMMANDS_FAILED.load(Ordering::Relaxed),
    }
}

/// Send a command and wake the event loop.
pub fn send_command(cmd: AppCommand) -> Result<(), CommandBridgeError> {
    let bridge = match load_active_bridge() {
        Ok(bridge) => bridge,
        Err(error) => {
            record_failed_command_send();
            return Err(error);
        }
    };

    let envelope = CommandEnvelope {
        trace_id: NEXT_COMMAND_TRACE_ID.fetch_add(1, Ordering::Relaxed),
        enqueued_at: Instant::now(),
        command: cmd,
    };

    COMMANDS_SENT.fetch_add(1, Ordering::Relaxed);
    bridge.sender.send(envelope).map_err(|_| {
        COMMANDS_SENT.fetch_sub(1, Ordering::Relaxed);
        record_failed_command_send();
        deactivate_and_clear(&bridge);
        CommandBridgeError::CommandChannelClosed
    })?;

    bridge
        .proxy
        .send_event(AppEvent::ProcessCommands)
        .map_err(|_| {
            record_failed_command_send();
            deactivate_and_clear(&bridge);
            CommandBridgeError::EventLoopClosed
        })?;
    Ok(())
}

/// Send a query command and block for a reply (5 second timeout).
pub fn send_query<T>(
    make_cmd: impl FnOnce(mpsc::Sender<T>) -> AppCommand,
) -> Result<T, CommandBridgeError> {
    let (reply_tx, reply_rx) = mpsc::channel();
    send_command(make_cmd(reply_tx))?;
    reply_rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|error| CommandBridgeError::ReplyTimeout(error.to_string()))
}
