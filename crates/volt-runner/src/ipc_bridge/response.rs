use volt_core::command::{self, AppCommand};
use volt_core::ipc::{
    IPC_HANDLER_ERROR_CODE, IPC_MAX_RESPONSE_BYTES, IpcResponse, response_script,
};

pub(super) fn send_response_to_window(js_window_id: &str, response: IpcResponse) {
    let Some(response_json) = response_json_for_window(response) else {
        return;
    };

    let script = response_script(&response_json);
    let _ = command::send_command(AppCommand::EvaluateScript {
        js_id: js_window_id.to_string(),
        script,
    });
}

fn response_json_for_window(response: IpcResponse) -> Option<String> {
    let response_id = response.id.clone();
    let mut response_json = match serde_json::to_string(&response) {
        Ok(serialized) => serialized,
        Err(error) => {
            let fallback = IpcResponse::error_with_code(
                response_id.clone(),
                format!("failed to serialize IPC response: {error}"),
                IPC_HANDLER_ERROR_CODE.to_string(),
            );
            match serde_json::to_string(&fallback) {
                Ok(serialized) => serialized,
                Err(_) => return None,
            }
        }
    };
    if response_json.len() > IPC_MAX_RESPONSE_BYTES {
        let fallback = IpcResponse::error_with_details(
            response_id,
            format!(
                "IPC response too large ({} bytes > {} bytes)",
                response_json.len(),
                IPC_MAX_RESPONSE_BYTES
            ),
            IPC_HANDLER_ERROR_CODE.to_string(),
            serde_json::json!({
                "responseBytes": response_json.len(),
                "maxResponseBytes": IPC_MAX_RESPONSE_BYTES
            }),
        );
        response_json = match serde_json::to_string(&fallback) {
            Ok(serialized) => serialized,
            Err(_) => return None,
        };
    }
    Some(response_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_responses_are_replaced_with_error_payloads() {
        let oversized = IpcResponse::success(
            "req-1".to_string(),
            serde_json::json!({
                "payload": "x".repeat(IPC_MAX_RESPONSE_BYTES + 1024)
            }),
        );

        let serialized = response_json_for_window(oversized).expect("serialized");
        assert!(serialized.contains("IPC response too large"));
        assert!(serialized.contains("maxResponseBytes"));
        assert!(serialized.len() < IPC_MAX_RESPONSE_BYTES);
    }
}
