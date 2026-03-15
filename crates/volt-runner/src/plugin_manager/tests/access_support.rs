use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::super::{AccessDialogRequest, PluginAccessPicker};

type PickerResponse = Result<Option<PathBuf>, String>;

#[derive(Clone, Default)]
pub(super) struct FakeAccessPicker {
    pub(super) seen: Arc<Mutex<Vec<AccessDialogRequest>>>,
    pub(super) responses: Arc<Mutex<Vec<PickerResponse>>>,
}

impl FakeAccessPicker {
    pub(super) fn from_responses(responses: Vec<PickerResponse>) -> Self {
        Self {
            seen: Arc::new(Mutex::new(Vec::new())),
            responses: Arc::new(Mutex::new(responses)),
        }
    }
}

impl PluginAccessPicker for FakeAccessPicker {
    fn pick_path(&self, request: AccessDialogRequest) -> Result<Option<PathBuf>, String> {
        self.seen.lock().expect("seen").push(request);
        self.responses.lock().expect("responses").remove(0)
    }
}
