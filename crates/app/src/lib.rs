//! The Tauri application. It is the only crate that knows every module: it holds
//! the commands, and (in later milestones) the session, permission checks,
//! workflows and background jobs.
//!
//! Page: `docs/05 Modules/Module - app.md`.

pub mod audit_actions;
pub mod commands;
pub mod migrations;

use tauri::Manager;

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
        .invoke_handler(tauri::generate_handler![commands::system::app_info])
        .run(tauri::generate_context!());
    // Start-up failures can't be shown in a window, because there is none yet.
    if let Err(error) = result {
        eprintln!("The app could not start: {error}");
        std::process::exit(1);
    }
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
