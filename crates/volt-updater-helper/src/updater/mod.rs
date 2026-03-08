#[cfg(target_os = "windows")]
pub(crate) mod apply;
pub(crate) mod args;
#[cfg(target_os = "windows")]
pub(crate) mod platform;

pub(crate) fn run() -> Result<(), String> {
    let args = args::parse_args()?;
    #[cfg(target_os = "windows")]
    {
        match args {
            args::Args::Install(args) => {
                let (target_path, staged_path) = platform::validate_install_paths(&args)?;
                platform::verify_invoker_process_matches_target(args.pid, &target_path)?;
                apply::log_info("waiting for app process to exit");
                platform::wait_for_process_exit(
                    args.pid,
                    std::time::Duration::from_secs(args.wait_timeout_secs),
                )?;
                apply::log_info("app exited, applying staged update");
                apply::replace_binary_with_retries(
                    &target_path,
                    &staged_path,
                    &args.expected_sha256,
                )?;
                apply::log_info("update completed");
            }
            args::Args::Rollback(args) => {
                let (target_path, backup_path, pending_marker_path) =
                    platform::validate_rollback_paths(&args)?;
                platform::verify_invoker_process_matches_target(args.pid, &target_path)?;
                apply::log_info("waiting for app process to exit before rollback");
                platform::wait_for_process_exit(
                    args.pid,
                    std::time::Duration::from_secs(args.wait_timeout_secs),
                )?;
                apply::log_info("app exited, restoring rollback candidate");
                apply::rollback_from_backup(
                    &target_path,
                    &backup_path,
                    &args.backup_sha256,
                    &pending_marker_path,
                )?;
                apply::log_info("rollback completed");
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = args;
        Err("volt-updater-helper is only supported on Windows".to_string())
    }
}
