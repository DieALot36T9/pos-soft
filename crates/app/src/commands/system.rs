//! System commands: app information (and later logs, audit and data checks).

use serde::Serialize;
use ts_rs::TS;

/// The app's name and version, shown in the window and on support reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

/// Tells the UI which app and version is running. Needs no permission.
#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Supershop POS".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_gives_the_crate_version() {
        let info = app_info();
        assert_eq!(info.name, "Supershop POS");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
    }
}
