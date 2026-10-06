//! The Tauri application. It is the only crate that knows every module: it holds
//! the commands, and (in later milestones) the session, permission checks,
//! workflows and background jobs.
//!
//! Page: `docs/05 Modules/Module - app.md`.

pub mod audit_actions;
pub mod audit_log;
pub mod commands;
pub mod dev_tools;
pub mod devices;
pub mod jobs;
pub mod logging;
pub mod migrations;
pub mod paths;
pub mod preview;
pub mod receipt_words;
pub mod session;
pub mod startup;
pub mod state;
pub mod workflows;

use std::sync::Arc;

use pos_core::{AppError, SystemClock};
use settings::LoggingSettings;
use tauri::{Manager, RunEvent, Wry};

use devices::Devices;
use paths::AppPaths;
use state::{AppState, DbHealth};

/// The window Tauri makes from `tauri.conf.json` when it has no label of its own.
const MAIN_WINDOW: &str = "main";

/// Starts the app: logging, the database, the backup timer, the window and every command.
pub fn run() {
    let built = tauri::Builder::default()
        // Must be the first plugin: a second copy of the app stops here and
        // brings the running copy's window to the front instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _folder| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_local_data_dir()?;
            let state = Arc::new(start_state(&data_dir)?);
            if state.health == DbHealth::Ok {
                jobs::start_backup_timer(Arc::clone(&state));
            }
            // Preview printouts are also saved as PNG files, in the app's own data folder.
            app.manage(Arc::new(Devices::new(Some(state.paths.prints()))));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(command_handler())
        .build(tauri::generate_context!());
    // Start-up failures can't be shown in a window, because there is none yet.
    let app = match built {
        Ok(app) => app,
        Err(error) => {
            eprintln!("The app could not start: {error}");
            std::process::exit(1);
        }
    };
    app.run(|handle, event| {
        if let RunEvent::Exit = event
            && let Some(state) = handle.try_state::<Arc<AppState>>()
        {
            jobs::backup_on_close(&state);
        }
    });
}

/// Starts logging, then opens and checks the database (Workflows, "Startup").
fn start_state(data_dir: &std::path::Path) -> Result<AppState, AppError> {
    let paths = AppPaths::new(data_dir);
    paths.create_folders()?;
    let log = logging::start_logging(&paths.logs(), false)?;
    let mut state = startup::start(paths, Arc::new(SystemClock))?;
    if state.health == DbHealth::Ok {
        let saved: LoggingSettings = state.db.read(|conn| Ok(settings::get(conn)?))?;
        log.set_detailed(saved.detailed)?;
    }
    state.log = Some(log);
    tracing::warn!(version = env!("CARGO_PKG_VERSION"), health = ?state.health, "app started");
    Ok(state)
}

/// What Tauri calls with each command from the window.
type CommandHandler = Box<dyn Fn(tauri::ipc::Invoke<Wry>) -> bool + Send + Sync>;

/// Every command the window may call: `commands::COMMANDS`, plus `commands::DEV_COMMANDS`
/// when the dev tools are built in. The two lists must match (permissions test).
fn command_handler() -> CommandHandler {
    if dev_tools::ON {
        with_dev_commands()
    } else {
        without_dev_commands()
    }
}

/// Every command, plus the dev-only ones.
// The list is one macro call: Tauri builds a single handler from it, so it can't be split.
#[allow(clippy::too_many_lines)]
fn with_dev_commands() -> CommandHandler {
    use commands::{
        audit, auth, backup, catalog, checkout, dev, hardware, inventory, payment_methods,
        settings, setup, shifts, startup, system, tax, users,
    };
    boxed(tauri::generate_handler![
        system::app_info,
        hardware::hardware_test_print,
        hardware::preview_take_printouts,
        hardware::print_window,
        startup::cmd::startup_status,
        startup::cmd::session_current,
        auth::cmd::login_tiles,
        auth::cmd::login,
        auth::cmd::logout,
        auth::cmd::lock,
        auth::cmd::unlock,
        auth::cmd::unlock_by_admin,
        auth::cmd::request_override,
        auth::cmd::change_own_secret,
        auth::cmd::recover_admin,
        startup::cmd::recovery_backups,
        startup::cmd::recovery_restore,
        setup::cmd::setup_status,
        setup::cmd::setup_save_shop,
        setup::cmd::setup_save_money,
        setup::cmd::setup_save_tax,
        setup::cmd::setup_save_admin,
        setup::cmd::setup_save_till,
        setup::cmd::setup_save_hardware,
        setup::cmd::setup_save_backup,
        setup::cmd::setup_finish,
        setup::cmd::setup_restore_backup,
        settings::cmd::currency_choices,
        settings::cmd::currency_format_for,
        settings::cmd::money_preview,
        settings::cmd::settings_get,
        settings::cmd::settings_save,
        settings::cmd::logs_set_detailed,
        tax::cmd::tax_rate_list,
        tax::cmd::tax_rate_save,
        tax::cmd::tax_rate_deactivate,
        backup::cmd::backup_now,
        backup::cmd::backup_list,
        backup::cmd::backup_restore,
        users::cmd::user_list,
        users::cmd::user_save,
        users::cmd::role_list,
        users::cmd::role_save,
        audit::cmd::audit_query,
        audit::cmd::audit_export,
        catalog::cmd::product_list,
        catalog::cmd::product_search,
        catalog::cmd::product_get,
        catalog::cmd::product_choices,
        catalog::cmd::product_save,
        catalog::cmd::product_archive,
        catalog::cmd::product_unarchive,
        catalog::cmd::product_delete,
        catalog::cmd::barcode_generate,
        catalog::cmd::variants_generate,
        catalog::cmd::kit_save,
        catalog::cmd::product_image_set,
        catalog::cmd::unit_list,
        catalog::cmd::unit_save,
        catalog::cmd::unit_archive,
        catalog::cmd::category_list,
        catalog::cmd::category_save,
        catalog::cmd::category_archive,
        catalog::cmd::brand_list,
        catalog::cmd::brand_save,
        catalog::cmd::brand_archive,
        inventory::cmd::stock_levels,
        inventory::cmd::stock_product,
        inventory::cmd::stock_lookup,
        inventory::cmd::stock_history,
        inventory::cmd::stock_reasons,
        inventory::cmd::stock_adjust,
        inventory::cmd::low_stock,
        inventory::cmd::near_expiry,
        inventory::cmd::stock_alerts,
        inventory::cmd::stock_verify,
        inventory::cmd::count_list,
        inventory::cmd::count_get,
        inventory::cmd::count_create,
        inventory::cmd::count_line,
        inventory::cmd::count_variances,
        inventory::cmd::count_post,
        inventory::cmd::count_cancel,
        checkout::cmd::checkout_state,
        checkout::cmd::checkout_scan,
        checkout::cmd::checkout_search,
        checkout::cmd::checkout_add_product,
        checkout::cmd::checkout_read_scale,
        checkout::cmd::checkout_add_weighed,
        checkout::cmd::checkout_set_qty,
        checkout::cmd::checkout_remove_line,
        checkout::cmd::checkout_hold,
        checkout::cmd::checkout_held_list,
        checkout::cmd::checkout_resume,
        checkout::cmd::checkout_cancel,
        checkout::cmd::checkout_payment_preview,
        checkout::cmd::checkout_complete,
        shifts::cmd::shift_current,
        shifts::cmd::shift_count_total,
        shifts::cmd::shift_open,
        shifts::cmd::shift_cash_move,
        shifts::cmd::shift_no_sale,
        shifts::cmd::shift_close,
        shifts::cmd::shift_force_close,
        payment_methods::cmd::payment_method_list,
        payment_methods::cmd::payment_method_save,
        dev::dev_fake_scale_get,
        dev::dev_fake_scale_set,
        dev::dev_fake_scale_read,
    ])
}

/// Every command, without the dev-only ones.
// The list is one macro call: Tauri builds a single handler from it, so it can't be split.
#[allow(clippy::too_many_lines)]
fn without_dev_commands() -> CommandHandler {
    use commands::{
        audit, auth, backup, catalog, checkout, hardware, inventory, payment_methods, settings,
        setup, shifts, startup, system, tax, users,
    };
    boxed(tauri::generate_handler![
        system::app_info,
        hardware::hardware_test_print,
        hardware::preview_take_printouts,
        hardware::print_window,
        startup::cmd::startup_status,
        startup::cmd::session_current,
        auth::cmd::login_tiles,
        auth::cmd::login,
        auth::cmd::logout,
        auth::cmd::lock,
        auth::cmd::unlock,
        auth::cmd::unlock_by_admin,
        auth::cmd::request_override,
        auth::cmd::change_own_secret,
        auth::cmd::recover_admin,
        startup::cmd::recovery_backups,
        startup::cmd::recovery_restore,
        setup::cmd::setup_status,
        setup::cmd::setup_save_shop,
        setup::cmd::setup_save_money,
        setup::cmd::setup_save_tax,
        setup::cmd::setup_save_admin,
        setup::cmd::setup_save_till,
        setup::cmd::setup_save_hardware,
        setup::cmd::setup_save_backup,
        setup::cmd::setup_finish,
        setup::cmd::setup_restore_backup,
        settings::cmd::currency_choices,
        settings::cmd::currency_format_for,
        settings::cmd::money_preview,
        settings::cmd::settings_get,
        settings::cmd::settings_save,
        settings::cmd::logs_set_detailed,
        tax::cmd::tax_rate_list,
        tax::cmd::tax_rate_save,
        tax::cmd::tax_rate_deactivate,
        backup::cmd::backup_now,
        backup::cmd::backup_list,
        backup::cmd::backup_restore,
        users::cmd::user_list,
        users::cmd::user_save,
        users::cmd::role_list,
        users::cmd::role_save,
        audit::cmd::audit_query,
        audit::cmd::audit_export,
        catalog::cmd::product_list,
        catalog::cmd::product_search,
        catalog::cmd::product_get,
        catalog::cmd::product_choices,
        catalog::cmd::product_save,
        catalog::cmd::product_archive,
        catalog::cmd::product_unarchive,
        catalog::cmd::product_delete,
        catalog::cmd::barcode_generate,
        catalog::cmd::variants_generate,
        catalog::cmd::kit_save,
        catalog::cmd::product_image_set,
        catalog::cmd::unit_list,
        catalog::cmd::unit_save,
        catalog::cmd::unit_archive,
        catalog::cmd::category_list,
        catalog::cmd::category_save,
        catalog::cmd::category_archive,
        catalog::cmd::brand_list,
        catalog::cmd::brand_save,
        catalog::cmd::brand_archive,
        inventory::cmd::stock_levels,
        inventory::cmd::stock_product,
        inventory::cmd::stock_lookup,
        inventory::cmd::stock_history,
        inventory::cmd::stock_reasons,
        inventory::cmd::stock_adjust,
        inventory::cmd::low_stock,
        inventory::cmd::near_expiry,
        inventory::cmd::stock_alerts,
        inventory::cmd::stock_verify,
        inventory::cmd::count_list,
        inventory::cmd::count_get,
        inventory::cmd::count_create,
        inventory::cmd::count_line,
        inventory::cmd::count_variances,
        inventory::cmd::count_post,
        inventory::cmd::count_cancel,
        checkout::cmd::checkout_state,
        checkout::cmd::checkout_scan,
        checkout::cmd::checkout_search,
        checkout::cmd::checkout_add_product,
        checkout::cmd::checkout_read_scale,
        checkout::cmd::checkout_add_weighed,
        checkout::cmd::checkout_set_qty,
        checkout::cmd::checkout_remove_line,
        checkout::cmd::checkout_hold,
        checkout::cmd::checkout_held_list,
        checkout::cmd::checkout_resume,
        checkout::cmd::checkout_cancel,
        checkout::cmd::checkout_payment_preview,
        checkout::cmd::checkout_complete,
        shifts::cmd::shift_current,
        shifts::cmd::shift_count_total,
        shifts::cmd::shift_open,
        shifts::cmd::shift_cash_move,
        shifts::cmd::shift_no_sale,
        shifts::cmd::shift_close,
        shifts::cmd::shift_force_close,
        payment_methods::cmd::payment_method_list,
        payment_methods::cmd::payment_method_save,
    ])
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
