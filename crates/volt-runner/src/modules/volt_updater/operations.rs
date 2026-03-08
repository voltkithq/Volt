use volt_core::updater::{self, UpdateConfig, UpdateInfo};

use super::config::current_app_version;
use super::events::{
    emit_update_lifecycle_telemetry, emit_update_progress_event, is_update_install_cancelled,
};
use super::state::{persist_pending_update_marker, remove_pending_update_marker};

pub(crate) fn download_and_install_with_public_key(
    info: UpdateInfo,
    public_key: String,
    operation_id: u64,
) -> Result<(), String> {
    let _ = emit_update_lifecycle_telemetry(&info.version, "install", "start", None);
    if is_update_install_cancelled(operation_id) {
        let _ = emit_update_lifecycle_telemetry(
            &info.version,
            "install",
            "cancelled",
            Some("cancelled before download"),
        );
        return Err("update installation cancelled".to_string());
    }

    let _ = emit_update_progress_event(&info.version, "download-started", 5);
    let _ = emit_update_lifecycle_telemetry(&info.version, "download", "start", None);
    let _ = emit_update_progress_event(&info.version, "downloading", 10);
    let config = UpdateConfig {
        endpoint: info.url.clone(),
        public_key,
        current_version: current_app_version(),
    };

    let binary = match updater::download_and_verify(&config, &info) {
        Ok(data) => data,
        Err(error) => {
            let detail = format!("download verification failed: {error}");
            let _ = emit_update_lifecycle_telemetry(
                &info.version,
                "download",
                "failure",
                Some(&detail),
            );
            return Err(format!("failed to download update binary: {error}"));
        }
    };
    let _ = emit_update_progress_event(&info.version, "downloaded", 80);
    let _ = emit_update_lifecycle_telemetry(&info.version, "download", "success", None);

    if is_update_install_cancelled(operation_id) {
        let _ = emit_update_lifecycle_telemetry(
            &info.version,
            "install",
            "cancelled",
            Some("cancelled after download"),
        );
        return Err("update installation cancelled".to_string());
    }

    let pending_marker_path =
        match persist_pending_update_marker(&info.version, &config.current_version, &info.sha256) {
            Ok(value) => value,
            Err(error) => {
                let detail = format!("failed to persist pending-update marker: {error}");
                let _ = emit_update_lifecycle_telemetry(
                    &info.version,
                    "marker",
                    "failure",
                    Some(&detail),
                );
                return Err(format!("failed to persist pending-update marker: {error}"));
            }
        };
    let _ = emit_update_lifecycle_telemetry(&info.version, "marker", "success", None);

    if let Err(error) = updater::apply_update(&binary) {
        if let Some(marker_path) = pending_marker_path.as_deref() {
            remove_pending_update_marker(marker_path);
        }
        let detail = format!("failed to install update: {error}");
        let _ = emit_update_lifecycle_telemetry(&info.version, "install", "failure", Some(&detail));
        return Err(format!("failed to install update: {error}"));
    }

    let _ = emit_update_progress_event(&info.version, "installed", 100);
    let _ = emit_update_lifecycle_telemetry(&info.version, "install", "success", None);
    Ok(())
}
