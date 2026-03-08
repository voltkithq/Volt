use super::{AppEvent, WindowLifecycleState, WindowStateStore, WindowStore};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use tao::event_loop::ControlFlow;

pub(super) fn remove_window_and_maybe_quit<F>(
    window_id: tao::window::WindowId,
    windows: &mut WindowStore,
    window_states: &mut WindowStateStore,
    js_to_tao: &mut HashMap<String, tao::window::WindowId>,
    tao_to_js: &mut HashMap<tao::window::WindowId, String>,
    control_flow: &mut ControlFlow,
    on_event: &mut F,
) -> bool
where
    F: FnMut(&AppEvent),
{
    let _ = set_window_closing(window_states, window_id);
    if windows.remove(&window_id).is_some() {
        set_window_closed(window_states, window_id);
        if let Some(js_id) = tao_to_js.remove(&window_id) {
            js_to_tao.remove(&js_id);
        }
        debug_assert_window_invariants(windows, js_to_tao, tao_to_js, window_states);
        on_event(&AppEvent::CloseWindow(window_id));
        if windows.is_empty() {
            *control_flow = ControlFlow::Exit;
            on_event(&AppEvent::Quit);
            return true;
        }
    } else {
        set_window_closed(window_states, window_id);
    }

    false
}

pub(super) fn set_window_active(
    window_states: &mut WindowStateStore,
    window_id: tao::window::WindowId,
) {
    window_states.insert(window_id, WindowLifecycleState::Active);
}

fn set_window_closing(
    window_states: &mut WindowStateStore,
    window_id: tao::window::WindowId,
) -> Result<(), String> {
    transition_window_state(window_states, window_id, WindowLifecycleState::Closing)
}

fn set_window_closed(window_states: &mut WindowStateStore, window_id: tao::window::WindowId) {
    let _ = transition_window_state(window_states, window_id, WindowLifecycleState::Closed);
}

pub(super) fn mark_all_windows_closed(
    window_states: &mut WindowStateStore,
    window_ids: impl Iterator<Item = tao::window::WindowId>,
) {
    for window_id in window_ids {
        set_window_closed(window_states, window_id);
    }
}

pub(super) fn transition_window_state<K>(
    window_states: &mut HashMap<K, WindowLifecycleState>,
    window_id: K,
    next: WindowLifecycleState,
) -> Result<(), String>
where
    K: Copy + Eq + Hash + std::fmt::Debug,
{
    match window_states.get(&window_id).copied() {
        Some(WindowLifecycleState::Active) => {
            if matches!(
                next,
                WindowLifecycleState::Closing | WindowLifecycleState::Closed
            ) {
                window_states.insert(window_id, next);
                Ok(())
            } else {
                Err(format!(
                    "invalid window state transition for {window_id:?}: Active -> {next:?}"
                ))
            }
        }
        Some(WindowLifecycleState::Closing) => {
            if matches!(next, WindowLifecycleState::Closed) {
                window_states.insert(window_id, next);
                Ok(())
            } else {
                Err(format!(
                    "invalid window state transition for {window_id:?}: Closing -> {next:?}"
                ))
            }
        }
        Some(WindowLifecycleState::Closed) => {
            if matches!(next, WindowLifecycleState::Closed) {
                Ok(())
            } else {
                Err(format!(
                    "invalid window state transition for {window_id:?}: Closed -> {next:?}"
                ))
            }
        }
        None => {
            if matches!(
                next,
                WindowLifecycleState::Closing | WindowLifecycleState::Closed
            ) {
                window_states.insert(window_id, next);
                Ok(())
            } else {
                Err(format!(
                    "invalid window state transition for {window_id:?}: <none> -> {next:?}"
                ))
            }
        }
    }
}

pub(super) fn debug_assert_window_invariants(
    windows: &WindowStore,
    js_to_tao: &HashMap<String, tao::window::WindowId>,
    tao_to_js: &HashMap<tao::window::WindowId, String>,
    window_states: &WindowStateStore,
) {
    let window_ids: HashSet<_> = windows.keys().copied().collect();
    if let Err(message) = check_window_invariants(&window_ids, js_to_tao, tao_to_js, window_states)
    {
        debug_assert!(false, "{message}");
    }
}

pub(super) fn check_window_invariants<K>(
    window_ids: &HashSet<K>,
    js_to_tao: &HashMap<String, K>,
    tao_to_js: &HashMap<K, String>,
    window_states: &HashMap<K, WindowLifecycleState>,
) -> Result<(), String>
where
    K: Copy + Eq + Hash + std::fmt::Debug,
{
    if window_ids.len() != js_to_tao.len() || window_ids.len() != tao_to_js.len() {
        return Err(format!(
            "window map length mismatch: windows={}, js_to_tao={}, tao_to_js={}",
            window_ids.len(),
            js_to_tao.len(),
            tao_to_js.len()
        ));
    }

    for (js_id, tao_id) in js_to_tao {
        if !window_ids.contains(tao_id) {
            return Err(format!(
                "js_to_tao references missing window: js_id={js_id}, tao_id={tao_id:?}"
            ));
        }
        let reverse = tao_to_js
            .get(tao_id)
            .ok_or_else(|| format!("missing reverse mapping for tao_id={tao_id:?}"))?;
        if reverse != js_id {
            return Err(format!(
                "mapping mismatch for tao_id={tao_id:?}: reverse={reverse}, expected={js_id}"
            ));
        }
    }

    for (tao_id, js_id) in tao_to_js {
        if !window_ids.contains(tao_id) {
            return Err(format!(
                "tao_to_js references missing window: tao_id={tao_id:?}, js_id={js_id}"
            ));
        }
        let forward = js_to_tao
            .get(js_id)
            .ok_or_else(|| format!("missing forward mapping for js_id={js_id}"))?;
        if forward != tao_id {
            return Err(format!(
                "mapping mismatch for js_id={js_id}: forward={forward:?}, expected={tao_id:?}"
            ));
        }
    }

    for window_id in window_ids {
        match window_states.get(window_id) {
            Some(WindowLifecycleState::Active) | Some(WindowLifecycleState::Closing) => {}
            Some(WindowLifecycleState::Closed) => {
                return Err(format!(
                    "window {window_id:?} is present in active map but state is Closed"
                ));
            }
            None => {
                return Err(format!(
                    "missing lifecycle state for active window {window_id:?}"
                ));
            }
        }
    }

    Ok(())
}
