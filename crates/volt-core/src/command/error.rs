use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CommandBridgeError {
    #[error("Bridge lock poisoned: {0}")]
    BridgeLockPoisoned(String),
    #[error("Bridge already initialized")]
    BridgeAlreadyInitialized,
    #[error("Event loop is not running")]
    EventLoopNotRunning,
    #[error("Command channel is closed")]
    CommandChannelClosed,
    #[error("Event loop is closed")]
    EventLoopClosed,
    #[error("Reply timeout: {0}")]
    ReplyTimeout(String),
}
