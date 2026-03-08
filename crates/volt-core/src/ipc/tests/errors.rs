use super::*;

#[test]
fn test_ipc_error_display_all_variants() {
    let error = IpcError::HandlerNotFound("missing".into());
    assert!(error.to_string().contains("missing"));

    let error = IpcError::InvalidMessage("bad json".into());
    assert!(error.to_string().contains("bad json"));

    let error = IpcError::PrototypePollution;
    assert!(error.to_string().contains("prototype pollution"));

    let error = IpcError::RateLimitExceeded(1000);
    assert!(error.to_string().contains("1000"));

    let error = IpcError::HandlerError("panic".into());
    assert!(error.to_string().contains("panic"));

    let error = IpcError::Security("blocked".into());
    assert!(error.to_string().contains("blocked"));

    let error = IpcError::PayloadTooLarge {
        size: IPC_MAX_REQUEST_BYTES + 1,
        max: IPC_MAX_REQUEST_BYTES,
    };
    assert!(error.to_string().contains("payload too large"));
}
