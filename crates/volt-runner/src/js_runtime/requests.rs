use std::sync::mpsc::Sender;
use std::time::Duration;

use serde_json::Value as JsonValue;
use volt_core::ipc::IpcResponse;

pub(super) enum RuntimeRequest {
    EvalI64 {
        script: String,
        response_tx: Sender<Result<i64, String>>,
    },
    #[cfg(test)]
    EvalBool {
        script: String,
        response_tx: Sender<Result<bool, String>>,
    },
    #[cfg(test)]
    EvalString {
        script: String,
        response_tx: Sender<Result<String, String>>,
    },
    #[cfg(test)]
    EvalPromiseI64 {
        script: String,
        response_tx: Sender<Result<i64, String>>,
    },
    #[cfg(test)]
    EvalPromiseString {
        script: String,
        response_tx: Sender<Result<String, String>>,
    },
    #[cfg(test)]
    EvalUnit {
        script: String,
        response_tx: Sender<Result<(), String>>,
    },
    LoadBackendBundle {
        script: String,
        response_tx: Sender<Result<(), String>>,
    },
    DispatchIpc {
        raw: String,
        timeout: Duration,
        response_tx: Sender<IpcResponse>,
    },
    DispatchNativeEvent {
        event_type: String,
        payload: JsonValue,
        response_tx: Option<Sender<Result<(), String>>>,
    },
    Shutdown,
}
