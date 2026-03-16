use volt_core::command::{self, AppCommand};
use volt_core::ipc::{
    IPC_HANDLER_ERROR_CODE, IPC_MAX_RESPONSE_BYTES, IpcResponse, response_script,
};

pub(super) fn send_response_to_window(js_window_id: &str, response: IpcResponse) {
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

    if response_json.len() > IPC_MAX_RESPONSE_BYTES {
        let truncated = IpcResponse::error_with_code(
            response.id,
            format!(
                "IPC response too large ({} bytes > {} bytes)",
                response_json.len(),
                IPC_MAX_RESPONSE_BYTES
            ),
            IPC_HANDLER_ERROR_CODE.to_string(),
        );
        let fallback_json = match serde_json::to_string(&truncated) {
            Ok(serialized) => serialized,
            Err(_) => return,
        };
        let script = response_script(&fallback_json);
        let _ = command::send_command(AppCommand::EvaluateScript {
            js_id: js_window_id.to_string(),
            script,
        });
        return;
    }

    let script = response_script(&response_json);
    let _ = command::send_command(AppCommand::EvaluateScript {
        js_id: js_window_id.to_string(),
        script,
    });
}
