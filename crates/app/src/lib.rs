//! The Tauri application. It is the only crate that knows every module: it holds
//! the commands, and (in later milestones) the session, permission checks,
//! workflows and background jobs.
//!
//! Page: `docs/05 Modules/Module - app.md`.

pub mod audit_actions;
pub mod commands;
pub mod dev_tools;
pub mod devices;
pub mod migrations;
pub mod preview;
pub mod receipt_words;
pub mod workflows;

use std::sync::Arc;

use tauri::{Manager, Wry};

use devices::Devices;

/// The window Tauri makes from `tauri.conf.json` when it has no label of its own.
const MAIN_WINDOW: &str = "main";

/// Starts the app: opens the window and registers every command.
pub fn run() {
    let result = tauri::Builder::default()
        // Must be the first plugin: a second copy of the app stops here and
        // brings the running copy's window to the front instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _folder| {
            show_main_window(app);
        }))
        .setup(|app| {
            // Preview printouts are also saved as PNG files, in the app's own data folder.
            let prints_folder = app
                .path()
                .app_local_data_dir()
                .ok()
                .map(|folder| folder.join("dev-prints"));
            app.manage(Arc::new(Devices::new(prints_folder)));
            Ok(())
        })
        .invoke_handler(command_handler())
        .run(tauri::generate_context!());
    // Start-up failures can't be shown in a window, because there is none yet.
    if let Err(error) = result {
        eprintln!("The app could not start: {error}");
        std::process::exit(1);
    }
}

/// What Tauri calls with each command from the window.
type CommandHandler = Box<dyn Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync>;

/// Every command the window may call: `commands::COMMANDS`, plus `commands::DEV_COMMANDS`
/// when the dev tools are built in.
fn command_handler() -> CommandHandler {
    use commands::{dev, hardware, system};
    if dev_tools::ON {
        boxed(tauri::generate_handler![
            system::app_info,
            hardware::hardware_test_print,
            hardware::preview_take_printouts,
            hardware::print_window,
            dev::dev_fake_scale_get,
            dev::dev_fake_scale_set,
            dev::dev_fake_scale_read,
        ])
    } else {
        boxed(tauri::generate_handler![
            system::app_info,
            hardware::hardware_test_print,
            hardware::preview_take_printouts,
            hardware::print_window,
        ])
    }
}

/// Lets the two command lists above have the same type.
fn boxed(
    handler: impl Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync + 'static,
) -> CommandHandler {
    Box::new(handler)
}

/// Brings the main window to the front, even if it was minimised.
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // If the window can't be raised, the shop simply sees the running copy as it was.
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
