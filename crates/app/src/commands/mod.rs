//! Tauri commands, one file per area of the UI. Commands stay thin: check the
//! permission, call one workflow or query, return.

pub mod audit;
pub mod auth;
pub mod backup;
pub mod dev;
pub mod hardware;
pub mod settings;
pub mod setup;
pub mod startup;
pub mod system;
pub mod tax;
pub mod users;

use pos_core::{AppError, Permission};

/// Every registered command and the permission it needs (`None` = anyone, even before login).
/// A new command must be added here and to `generate_handler!` in `lib.rs` (Security Model).
///
/// Some commands have their own rule instead of a permission:
/// - `setup_*` work only while setup isn't complete (nobody exists yet to sign in).
/// - `recovery_*` work only while the database is damaged (BAK-06).
/// - `settings_get`, `settings_save` and `tax_rate_*` also work without sign-in
///   while setup is open, because the wizard uses them.
/// - "Test print" goes to the Preview printer and reaches no device yet; when printer
///   settings are used (C-M7), `hardware_test_print` needs `SettingsManage`.
/// - `login_tiles`, `login` and `recover_admin` work only once setup is complete.
/// - `lock`, `unlock`, `unlock_by_admin`, `request_override` and `change_own_secret`
///   need someone signed in (any role); `unlock*` only while the till is locked.
/// - `backup_restore` also takes an admin's approval token (`require_or_approval`, USR-05).
pub const COMMANDS: &[(&str, Option<Permission>)] = &[
    ("app_info", None),
    ("hardware_test_print", None),
    ("preview_take_printouts", None),
    ("print_window", None),
    ("startup_status", None),
    ("session_current", None),
    ("login_tiles", None),
    ("login", None),
    ("logout", None),
    ("lock", None),
    ("unlock", None),
    ("unlock_by_admin", None),
    ("request_override", None),
    ("change_own_secret", None),
    ("recover_admin", None),
    ("recovery_backups", None),
    ("recovery_restore", None),
    ("setup_status", None),
    ("setup_save_shop", None),
    ("setup_save_money", None),
    ("setup_save_tax", None),
    ("setup_save_admin", None),
    ("setup_save_till", None),
    ("setup_save_hardware", None),
    ("setup_save_backup", None),
    ("setup_finish", None),
    ("setup_restore_backup", None),
    ("currency_choices", None),
    ("currency_format_for", None),
    ("money_preview", None),
    ("settings_get", Some(Permission::SettingsManage)),
    ("settings_save", Some(Permission::SettingsManage)),
    ("logs_set_detailed", Some(Permission::LogsManage)),
    ("tax_rate_list", Some(Permission::TaxManage)),
    ("tax_rate_save", Some(Permission::TaxManage)),
    ("tax_rate_deactivate", Some(Permission::TaxManage)),
    ("backup_now", Some(Permission::BackupRun)),
    ("backup_list", Some(Permission::BackupRun)),
    ("backup_restore", Some(Permission::BackupRestore)),
    ("user_list", Some(Permission::UsersManage)),
    ("user_save", Some(Permission::UsersManage)),
    ("role_list", Some(Permission::UsersManage)),
    ("role_save", Some(Permission::UsersManage)),
    ("audit_query", Some(Permission::AuditView)),
    ("audit_export", Some(Permission::AuditView)),
];

/// The dev-only commands (fake scale panel). Registered only when `dev_tools::ON` is true.
/// They need no permission: they only change the pretend scale.
pub const DEV_COMMANDS: &[(&str, Option<Permission>)] = &[
    ("dev_fake_scale_get", None),
    ("dev_fake_scale_set", None),
    ("dev_fake_scale_read", None),
];

/// Runs a command's work on a worker thread, so the window never waits, inside a
/// log span named after the command. A panic becomes `AppError::Internal`, and an
/// internal error is logged with its details (the screen shows only its reference).
pub(crate) async fn on_worker<T, F>(name: &'static str, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    let result = tauri::async_runtime::spawn_blocking(move || {
        let span = tracing::info_span!("cmd", name);
        let _entered = span.enter();
        let started = std::time::Instant::now();
        let result = work();
        tracing::info!(
            ms = started.elapsed().as_millis() as u64,
            ok = result.is_ok(),
            "done"
        );
        result
    })
    .await
    .map_err(|error| AppError::internal(format!("{name} stopped: {error}")))?;
    if let Err(AppError::Internal { id, detail }) = &result {
        tracing::error!(command = name, id, detail, "internal error");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[(&'static str, Option<Permission>)]) -> Vec<&'static str> {
        list.iter().map(|(name, _)| *name).collect()
    }

    #[test]
    fn every_command_has_a_permission_decision() {
        let all = names(COMMANDS);
        assert_eq!(all.len(), 45);
        let mut unique = all.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), all.len(), "a command is listed twice");
        assert_eq!(
            names(DEV_COMMANDS),
            vec![
                "dev_fake_scale_get",
                "dev_fake_scale_set",
                "dev_fake_scale_read"
            ]
        );
    }

    #[test]
    fn app_info_needs_no_login() {
        assert_eq!(COMMANDS[0], ("app_info", None));
    }

    #[test]
    fn set_13_settings_tax_backup_and_logs_need_their_permission() {
        let needs = |name: &str| {
            COMMANDS
                .iter()
                .find(|(other, _)| *other == name)
                .map(|(_, p)| *p)
        };
        assert_eq!(
            needs("settings_save"),
            Some(Some(Permission::SettingsManage))
        );
        assert_eq!(needs("tax_rate_save"), Some(Some(Permission::TaxManage)));
        assert_eq!(needs("backup_now"), Some(Some(Permission::BackupRun)));
        assert_eq!(
            needs("backup_restore"),
            Some(Some(Permission::BackupRestore))
        );
        assert_eq!(
            needs("logs_set_detailed"),
            Some(Some(Permission::LogsManage))
        );
    }

    #[test]
    fn hw_10_printing_without_a_device_needs_no_login() {
        for name in [
            "hardware_test_print",
            "preview_take_printouts",
            "print_window",
        ] {
            assert!(COMMANDS.contains(&(name, None)), "{name}");
        }
    }

    #[test]
    fn hw_10_dev_commands_are_kept_apart_and_need_no_login() {
        for (name, permission) in DEV_COMMANDS {
            assert!(name.starts_with("dev_"), "{name}");
            assert_eq!(*permission, None, "{name}");
            assert!(!COMMANDS.iter().any(|(other, _)| other == name), "{name}");
        }
    }
}
