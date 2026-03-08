use boa_engine::object::builtins::JsFunction;
use boa_engine::{Context, IntoJsFunctionCopied, JsResult, JsValue, Module};
use volt_core::command::{self, AppCommand};
use volt_core::permissions::Permission;

use super::{
    bind_native_event_off, bind_native_event_on, native_function_module,
    normalize_single_event_name, promise_from_result, require_permission_message,
};

const SHORTCUT_TRIGGERED_EVENT: &str = "shortcut:triggered";

fn normalize_shortcut_event_name(event_name: String) -> Result<&'static str, String> {
    normalize_single_event_name(
        "shortcut",
        event_name,
        "triggered",
        SHORTCUT_TRIGGERED_EVENT,
    )
}

fn register_shortcut(accelerator: String) -> Result<u32, String> {
    require_permission_message(Permission::GlobalShortcut)?;
    command::send_query(|reply| AppCommand::RegisterShortcut { accelerator, reply })
        .map_err(|error| format!("failed to send shortcut registration command: {error}"))?
        .map_err(|error| format!("failed to register shortcut: {error}"))
}

fn unregister_shortcut(accelerator: String) -> Result<(), String> {
    require_permission_message(Permission::GlobalShortcut)?;
    command::send_query(|reply| AppCommand::UnregisterShortcut { accelerator, reply })
        .map_err(|error| format!("failed to send shortcut unregistration command: {error}"))?
        .map_err(|error| format!("failed to unregister shortcut: {error}"))
}

fn unregister_all_shortcuts() -> Result<(), String> {
    require_permission_message(Permission::GlobalShortcut)?;
    command::send_query(|reply| AppCommand::UnregisterAllShortcuts { reply })
        .map_err(|error| format!("failed to send unregister-all command: {error}"))?
        .map_err(|error| format!("failed to unregister all shortcuts: {error}"))
}

fn register(accelerator: String, context: &mut Context) -> JsValue {
    promise_from_result(context, register_shortcut(accelerator)).into()
}

fn unregister(accelerator: String, context: &mut Context) -> JsValue {
    promise_from_result(context, unregister_shortcut(accelerator)).into()
}

fn unregister_all(context: &mut Context) -> JsValue {
    promise_from_result(context, unregister_all_shortcuts()).into()
}

fn on(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_on(
        context,
        "volt:globalShortcut",
        Permission::GlobalShortcut,
        event_name,
        handler,
        normalize_shortcut_event_name,
    )
}

fn off(event_name: String, handler: JsFunction, context: &mut Context) -> JsResult<()> {
    bind_native_event_off(
        context,
        "volt:globalShortcut",
        Permission::GlobalShortcut,
        event_name,
        handler,
        normalize_shortcut_event_name,
    )
}

pub fn build_module(context: &mut Context) -> Module {
    let register = register.into_js_function_copied(context);
    let unregister = unregister.into_js_function_copied(context);
    let unregister_all = unregister_all.into_js_function_copied(context);
    let on = on.into_js_function_copied(context);
    let off = off.into_js_function_copied(context);

    native_function_module(
        context,
        vec![
            ("register", register),
            ("unregister", unregister),
            ("unregisterAll", unregister_all),
            ("on", on),
            ("off", off),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::test_utils::{init_test_bridge, shutdown_test_bridge, test_guard};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn normalize_shortcut_event_name_accepts_triggered_only() {
        assert_eq!(
            normalize_shortcut_event_name("triggered".to_string()),
            Ok("shortcut:triggered")
        );
        assert!(normalize_shortcut_event_name("".to_string()).is_err());
        assert!(normalize_shortcut_event_name("click".to_string()).is_err());
    }

    #[test]
    fn register_and_unregister_dispatch_expected_commands() {
        let _guard = test_guard();
        let (receiver, lifecycle, _proxy) = init_test_bridge();
        crate::modules::configure(crate::modules::ModuleConfig {
            fs_base_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            permissions: vec!["globalShortcut".to_string()],
            ..Default::default()
        })
        .expect("configure module permissions");

        let responder = thread::spawn(move || {
            let first = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("register");
            match first.command {
                AppCommand::RegisterShortcut { accelerator, reply } => {
                    assert_eq!(accelerator, "CmdOrCtrl+Shift+P");
                    let _ = reply.send(Ok(42));
                }
                command => panic!("unexpected command: {command:?}"),
            }

            let second = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("unregister");
            match second.command {
                AppCommand::UnregisterShortcut { accelerator, reply } => {
                    assert_eq!(accelerator, "CmdOrCtrl+Shift+P");
                    let _ = reply.send(Ok(()));
                }
                command => panic!("unexpected command: {command:?}"),
            }

            let third = receiver
                .recv_timeout(Duration::from_secs(1))
                .expect("unregister all");
            match third.command {
                AppCommand::UnregisterAllShortcuts { reply } => {
                    let _ = reply.send(Ok(()));
                }
                command => panic!("unexpected command: {command:?}"),
            }
        });

        let shortcut_id =
            register_shortcut("CmdOrCtrl+Shift+P".to_string()).expect("register shortcut");
        assert_eq!(shortcut_id, 42);
        unregister_shortcut("CmdOrCtrl+Shift+P".to_string()).expect("unregister shortcut");
        unregister_all_shortcuts().expect("unregister all shortcuts");

        shutdown_test_bridge(lifecycle);
        let _ = responder.join();
    }

    #[test]
    fn register_shortcut_requires_permission() {
        crate::modules::configure(crate::modules::ModuleConfig {
            fs_base_dir: std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            permissions: Vec::new(),
            ..Default::default()
        })
        .expect("configure module permissions");

        let result = register_shortcut("CmdOrCtrl+K".to_string());
        assert!(result.is_err());
        assert!(
            result
                .err()
                .is_some_and(|message| message.contains("Permission denied"))
        );
    }
}
