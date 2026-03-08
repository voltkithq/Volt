#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod logging;
mod updater;

fn main() {
    logging::init_logging(if cfg!(debug_assertions) {
        "debug"
    } else {
        "warn"
    });
    if let Err(error) = updater::run() {
        tracing::error!(error = %error, "updater helper failed");
        std::process::exit(1);
    }
}
