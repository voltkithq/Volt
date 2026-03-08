use super::*;

#[test]
fn test_downgrade_prevention_matrix_rejects_equal_or_older_versions() {
    let cases = [
        ("1.0.0", "1.0.0"),
        ("1.0.0", "0.9.9"),
        ("2.3.4", "2.3.3"),
        ("2.3.4", "2.3.4-rc.1"),
    ];

    for (current, offered) in cases {
        let response = serde_json::to_string(&signed_update_info(
            offered,
            "https://updates.example.com/app.exe",
            &"ab".repeat(32),
        ))
        .expect("serialize update info");
        let result =
            check_for_update_against_single_response_with_current("200 OK", &response, current);
        match result {
            Err(UpdateError::DowngradePrevented {
                current: current_reported,
                offered: offered_reported,
            }) => {
                assert_eq!(current_reported, current.to_string());
                assert_eq!(offered_reported, offered.to_string());
            }
            other => panic!(
                "expected downgrade prevention for current={current} offered={offered}, got {other:?}"
            ),
        }
    }
}

#[test]
fn test_check_for_update_accepts_higher_semver_versions() {
    let response = serde_json::to_string(&signed_update_info(
        "1.0.1",
        "https://updates.example.com/app.exe",
        &"ab".repeat(32),
    ))
    .expect("serialize update info");
    let result =
        check_for_update_against_single_response_with_current("200 OK", &response, "1.0.0")
            .expect("newer version should be accepted");
    assert!(result.is_some());
}

#[test]
fn test_check_for_update_rejects_tampered_signed_metadata() {
    let mut info = signed_update_info(
        "1.0.1",
        "https://updates.example.com/app.exe",
        &"ab".repeat(32),
    );
    info.version = "1.0.2".to_string();

    let response = serde_json::to_string(&info).expect("serialize tampered update info");
    let result =
        check_for_update_against_single_response_with_current("200 OK", &response, "1.0.0");

    assert!(matches!(result, Err(UpdateError::SignatureInvalid(_))));
}

#[test]
fn test_check_for_update_204_returns_none() {
    let result = check_for_update_against_single_response("204 No Content", "")
        .expect("check_for_update should treat 204 as no update");
    assert!(result.is_none());
}

#[test]
fn test_check_for_update_404_returns_none() {
    let result = check_for_update_against_single_response("404 Not Found", "")
        .expect("check_for_update should treat 404 as no update");
    assert!(result.is_none());
}
