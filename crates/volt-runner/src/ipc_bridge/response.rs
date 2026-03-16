use volt_core::command::{self, AppCommand};
use volt_core::ipc::{IPC_HANDLER_ERROR_CODE, IpcResponse, response_script};

pub(super) fn send_response_to_window(js_window_id: &str, response: IpcResponse) {
    let request_id = response.id.clone();
    let response_json = match serde_json::to_string(&response) {
        Ok(serialized) => serialized,
        Err(error) => {
            let fallback = IpcResponse::error_with_code(
                response.id,
                format!("failed to serialize IPC response: {error}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            );
            match serde_json::to_string(&fallback) {
                Ok(serialized) => serialized,
                Err(_) => return,
            }
        }
    };

    let script = response_script(&response_json);
    if let Err(error) = command::send_command(AppCommand::EvaluateScript {
        js_id: js_window_id.to_string(),
        script,
    }) {
        tracing::debug!(
            window_id = %js_window_id,
            request_id = %request_id,
            error = %error,
            "dropping IPC response because the target window is unavailable"
        );
    }
}
