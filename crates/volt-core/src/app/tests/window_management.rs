use crate::app;
use std::collections::{HashMap, HashSet};

#[test]
fn test_parse_hotkey_accelerator() {
    let parsed = app::command_handling::parse_hotkey_accelerator("CmdOrCtrl+Shift+P");
    assert!(parsed.is_ok());
}

#[test]
fn test_parse_hotkey_invalid() {
    let parsed = app::command_handling::parse_hotkey_accelerator("CmdOrCtrl+Shift+NopeKey");
    assert!(parsed.is_err());
}

#[test]
fn test_begin_shutdown_cleanup_sets_flag_once() {
    let mut done = false;
    assert!(app::command_handling::begin_shutdown_cleanup(&mut done));
    assert!(done);
    assert!(!app::command_handling::begin_shutdown_cleanup(&mut done));
}

#[test]
fn test_window_state_machine_valid_transitions() {
    let mut states: HashMap<u64, app::WindowLifecycleState> = HashMap::new();

    assert_eq!(
        app::window_management::transition_window_state(
            &mut states,
            1,
            app::WindowLifecycleState::Closing
        ),
        Ok(())
    );
    assert_eq!(
        app::window_management::transition_window_state(
            &mut states,
            1,
            app::WindowLifecycleState::Closed
        ),
        Ok(())
    );
}

#[test]
fn test_window_state_machine_invalid_transition_from_closed() {
    let mut states: HashMap<u64, app::WindowLifecycleState> = HashMap::new();
    states.insert(7, app::WindowLifecycleState::Closed);

    let transition = app::window_management::transition_window_state(
        &mut states,
        7,
        app::WindowLifecycleState::Active,
    );
    assert!(transition.is_err());
}

#[test]
fn test_window_invariants_happy_path() {
    let window_ids: HashSet<u64> = HashSet::from([1, 2]);
    let js_to_tao: HashMap<String, u64> =
        HashMap::from([("window-1".to_string(), 1), ("window-2".to_string(), 2)]);
    let tao_to_js: HashMap<u64, String> =
        HashMap::from([(1, "window-1".to_string()), (2, "window-2".to_string())]);
    let states: HashMap<u64, app::WindowLifecycleState> = HashMap::from([
        (1, app::WindowLifecycleState::Active),
        (2, app::WindowLifecycleState::Closing),
    ]);

    assert!(
        app::window_management::check_window_invariants(
            &window_ids,
            &js_to_tao,
            &tao_to_js,
            &states
        )
        .is_ok()
    );
}

#[test]
fn test_window_invariants_reject_closed_active_window() {
    let window_ids: HashSet<u64> = HashSet::from([11]);
    let js_to_tao: HashMap<String, u64> = HashMap::from([("window-11".to_string(), 11)]);
    let tao_to_js: HashMap<u64, String> = HashMap::from([(11, "window-11".to_string())]);
    let states: HashMap<u64, app::WindowLifecycleState> =
        HashMap::from([(11, app::WindowLifecycleState::Closed)]);

    let result = app::window_management::check_window_invariants(
        &window_ids,
        &js_to_tao,
        &tao_to_js,
        &states,
    );
    assert!(result.is_err());
}
