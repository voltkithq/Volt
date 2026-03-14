mod child;
mod io;
mod wire;

pub(in crate::plugin_manager) use self::child::RealPluginProcessFactory;
pub(in crate::plugin_manager) use self::wire::WireMessage;
#[cfg(test)]
pub(in crate::plugin_manager) use self::wire::{WireError, WireMessageType};
