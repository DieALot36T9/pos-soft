//! The Tauri application. It is the only crate that knows every module: it holds
//! the commands, and (in later milestones) the session, permission checks,
//! workflows and background jobs.
//!
//! Page: `docs/05 Modules/Module - app.md`.

pub mod commands;
pub mod migrations;

/// Starts the app: opens the window and registers every command.
pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::system::app_info])
        .run(tauri::generate_context!());
    // Start-up failures can't be shown in a window, because there is none yet.
    if let Err(error) = result {
        eprintln!("The app could not start: {error}");
        std::process::exit(1);
    }
}
