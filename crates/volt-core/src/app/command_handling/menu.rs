use super::super::WindowStore;

pub(super) fn apply_menu_to_windows(
    windows: &WindowStore,
    items: &[crate::menu::MenuItemConfig],
    app_menu: &mut Option<muda::Menu>,
) -> Result<(), String> {
    let menu =
        crate::menu::build_menu(items).map_err(|e| format!("Failed to build app menu: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        use tao::platform::windows::WindowExtWindows;

        for (window, _) in windows.values() {
            // SAFETY: `hwnd()` comes from a live tao window owned by this event loop thread.
            unsafe {
                menu.init_for_hwnd(window.inner().hwnd() as _)
                    .map_err(|e| format!("Failed to attach menu to HWND: {e}"))?;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        menu.init_for_nsapp();
    }

    #[cfg(target_os = "linux")]
    {
        use tao::platform::unix::WindowExtUnix;

        for (window, _) in windows.values() {
            let gtk_window = window.inner().gtk_window();
            menu.init_for_gtk_window(gtk_window, None::<&gtk::Container>)
                .map_err(|e| format!("Failed to attach menu to GTK window: {e}"))?;
        }
    }

    *app_menu = Some(menu);
    Ok(())
}
