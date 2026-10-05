//! Tauri commands, one file per area of the UI. Commands stay thin: check the
//! permission, call one workflow or query, return.

pub mod dev;
pub mod hardware;
pub mod system;

use pos_core::{AppError, Permission};

/// Every registered command and the permission it needs (`None` = anyone, even before login).
/// A new command must be added here and to `generate_handler!` in `lib.rs` (Security Model).
///
/// No command here needs a permission yet, because none reaches a device or the shop's data:
/// "Test print" goes to the Preview printer, and the print dialog prints what the
/// screen already shows. When printer settings arrive (C-M2, C-M7), `hardware_test_print`
/// needs `SettingsManage`, except during first-run setup.
pub const COMMANDS: &[(&str, Option<Permission>)] = &[
    ("app_info", None),
    ("hardware_test_print", None),
    ("preview_take_printouts", None),
    ("print_window", None),
];

/// The dev-only commands (fake scale panel). Registered only when `dev_tools::ON` is true.
/// They need no permission: they only change the pretend scale.
pub const DEV_COMMANDS: &[(&str, Option<Permission>)] = &[
    ("dev_fake_scale_get", None),
    ("dev_fake_scale_set", None),
    ("dev_fake_scale_read", None),
];

/// Runs slow work (drawing, devices) on a worker thread, so the window never waits.
/// A panic becomes `AppError::Internal`.
pub(crate) async fn on_worker<T, F>(work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::internal(format!("a command stopped: {error}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[(&'static str, Option<Permission>)]) -> Vec<&'static str> {
        list.iter().map(|(name, _)| *name).collect()
    }

    #[test]
    fn every_command_has_a_permission_decision() {
        assert_eq!(
            names(COMMANDS),
            vec![
                "app_info",
                "hardware_test_print",
                "preview_take_printouts",
                "print_window"
            ]
        );
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
