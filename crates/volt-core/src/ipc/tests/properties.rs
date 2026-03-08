use proptest::prelude::*;
use proptest::string::string_regex;

use super::helpers::{extract_event_payload, extract_response_payload};
use super::*;

proptest! {
    #[test]
    fn prop_response_script_roundtrip(raw in any::<String>()) {
        let script = response_script(&raw);
        prop_assert!(!script.contains(char::from_u32(0x2028).expect("u2028")));
        prop_assert!(!script.contains(char::from_u32(0x2029).expect("u2029")));

        let decoded = extract_response_payload(&script)
            .expect("script payload should parse");
        prop_assert_eq!(decoded, raw);
    }

    #[test]
    fn prop_event_script_roundtrip(
        event in any::<String>(),
        payload in any::<String>(),
    ) {
        let data = serde_json::Value::String(payload);
        let script = event_script(&event, &data)
            .expect("event script should serialize");

        prop_assert!(!script.contains(char::from_u32(0x2028).expect("u2028")));
        prop_assert!(!script.contains(char::from_u32(0x2029).expect("u2029")));

        let (decoded_event, decoded_data) = extract_event_payload(&script)
            .expect("event payload should parse");
        prop_assert_eq!(decoded_event, event);
        prop_assert_eq!(decoded_data, data);
    }

    #[test]
    fn prop_handle_message_never_panics(input in any::<String>()) {
        let registry = IpcRegistry::new();
        let _ = registry.handle_message(&input);
    }

    #[test]
    fn prop_valid_request_preserves_id(
        id in string_regex("[A-Za-z0-9-]{0,32}").expect("regex"),
        method in string_regex("[A-Za-z0-9._-]{1,32}").expect("regex"),
        arg in any::<i64>(),
    ) {
        let registry = IpcRegistry::new();
        let raw = serde_json::to_string(&serde_json::json!({
            "id": id,
            "method": method,
            "args": { "value": arg }
        }))
        .expect("request serialization");

        let response = registry
            .handle_message(&raw)
            .expect("valid request should not be rejected by parser");
        let parsed: IpcResponse = serde_json::from_str(&response)
            .expect("response should deserialize");
        prop_assert_eq!(parsed.id, id);
    }
}
