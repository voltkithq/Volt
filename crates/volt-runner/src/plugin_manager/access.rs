use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use volt_core::dialog::{self, OpenDialogOptions};

pub(crate) trait PluginAccessPicker: Send + Sync {
    fn pick_path(&self, request: AccessDialogRequest) -> Result<Option<PathBuf>, String>;
}

#[derive(Debug, Clone)]
pub(crate) struct AccessDialogRequest {
    pub(crate) title: String,
    pub(crate) directory: bool,
    pub(crate) multiple: bool,
}

pub(crate) struct NativePluginAccessPicker;

impl PluginAccessPicker for NativePluginAccessPicker {
    fn pick_path(&self, request: AccessDialogRequest) -> Result<Option<PathBuf>, String> {
        let (sender, receiver) = mpsc::channel();
        let _ = thread::Builder::new()
            .name("volt-plugin-access-dialog".to_string())
            .spawn(move || {
                let selection = dialog::show_open_dialog(&OpenDialogOptions {
                    title: Some(request.title),
                    multiple: request.multiple,
                    directory: request.directory,
                    ..OpenDialogOptions::default()
                })
                .into_iter()
                .next();
                let _ = sender.send(selection);
            })
            .map_err(|error| error.to_string())?;

        receiver.recv().map_err(|error| error.to_string())
    }
}
