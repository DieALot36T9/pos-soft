//! Tauri commands, one file per area of the UI. Commands stay thin: check the
//! permission, call one workflow or query, return.

pub mod system;

use pos_core::Permission;

/// Every registered command and the permission it needs (`None` = anyone, even before login).
/// A new command must be added here and to `generate_handler!` in `lib.rs` (Security Model).
pub const COMMANDS: &[(&str, Option<Permission>)] = &[("app_info", None)];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_permission_decision() {
        let names: Vec<&str> = COMMANDS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec!["app_info"]);
    }

    #[test]
    fn app_info_needs_no_login() {
        assert_eq!(COMMANDS[0], ("app_info", None));
    }
}
