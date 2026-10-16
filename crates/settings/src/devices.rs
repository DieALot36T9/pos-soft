//! Settings for devices, barcodes, backups and logging.
//! These only store what the shop chose; the `hardware`, `barcode` and `db` modules use them.

use pos_core::AppError;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::Setting;
use crate::setting::{invalid, is_filled};

/// How to reach a printer. Mirrors `hardware::PrinterConfig`; `app` turns one into the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum PrinterConnection {
    /// A printer installed in Windows. Example: "EPSON TM-T82".
    WindowsSpooler { printer_name: String },
    /// A network printer. Example: 192.168.1.50, port 9100.
    Tcp { host: String, port: u16 },
    /// A serial port. Example: "COM3" at 9600.
    Serial { port: String, baud: u32 },
    /// A CUPS queue on macOS or Linux.
    Cups { queue: String },
    /// No printer: the receipt picture is shown on screen.
    Preview,
    /// For development: the bytes are written to a file.
    File { path: String },
}

impl PrinterConnection {
    /// True when every detail the connection needs is filled in.
    fn is_complete(&self) -> bool {
        match self {
            PrinterConnection::WindowsSpooler { printer_name } => is_filled(printer_name),
            PrinterConnection::Tcp { host, port } => is_filled(host) && *port > 0,
            PrinterConnection::Serial { port, baud } => is_filled(port) && *baud > 0,
            PrinterConnection::Cups { queue } => is_filled(queue),
            PrinterConnection::Preview => true,
            PrinterConnection::File { path } => is_filled(path),
        }
    }
}

/// Which pin of the receipt printer opens the cash drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum DrawerPin {
    Pin2,
    Pin5,
}

/// Where the scale is plugged in and which built-in profile reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct ScaleSettings {
    /// Example: "COM3" or "/dev/cu.usbserial-1".
    pub port: String,
    pub baud: u32,
    /// The name of a profile from `hardware::builtin_scale_profiles`. Example: "cas".
    pub profile: String,
}

impl Default for ScaleSettings {
    fn default() -> ScaleSettings {
        ScaleSettings {
            port: String::new(),
            baud: 9600,
            profile: String::new(),
        }
    }
}

/// The shop's devices (rule SET-10). `None` means "not set up yet".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct HardwareSettings {
    pub receipt_printer: Option<PrinterConnection>,
    pub drawer_pin: DrawerPin,
    pub scale: Option<ScaleSettings>,
    pub label_printer: Option<PrinterConnection>,
}

impl Default for HardwareSettings {
    fn default() -> HardwareSettings {
        HardwareSettings {
            receipt_printer: None,
            drawer_pin: DrawerPin::Pin2,
            scale: None,
            label_printer: None,
        }
    }
}

impl Setting for HardwareSettings {
    const KEY: &'static str = "hardware";

    fn validate(&self) -> Result<(), AppError> {
        if let Some(printer) = &self.receipt_printer
            && !printer.is_complete()
        {
            return Err(invalid("receipt_printer", "printer_connection_incomplete"));
        }
        if let Some(printer) = &self.label_printer
            && !printer.is_complete()
        {
            return Err(invalid("label_printer", "printer_connection_incomplete"));
        }
        if let Some(scale) = &self.scale {
            if !is_filled(&scale.port) || scale.baud == 0 {
                return Err(invalid("scale", "scale_port_missing"));
            }
            if !is_filled(&scale.profile) {
                return Err(invalid("scale", "scale_profile_missing"));
            }
        }
        Ok(())
    }
}

/// What a scale sticker barcode carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ScaleKind {
    Weight,
    Price,
}

/// One scale sticker rule. Same shape as `barcode::ScaleRule`; `app` copies one to the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScaleRule {
    /// Example: "Weight, 3 decimals".
    pub name: String,
    /// Odoo nomenclature pattern. Example: "21.....{NNDDD}".
    pub pattern: String,
    pub kind: ScaleKind,
}

/// The shop's scale sticker rules. Full pattern checks are done by `barcode::validate_rule`,
/// which `app` calls before saving.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(transparent)]
#[ts(export)]
pub struct ScaleRules(pub Vec<ScaleRule>);

impl Default for ScaleRules {
    /// The two rules from `Module - barcode`: weight under prefix 21, price under 23.
    fn default() -> ScaleRules {
        ScaleRules(vec![
            ScaleRule {
                name: "Weight, 3 decimals".to_string(),
                pattern: "21.....{NNDDD}".to_string(),
                kind: ScaleKind::Weight,
            },
            ScaleRule {
                name: "Price, 2 decimals".to_string(),
                pattern: "23.....{NNNDD}".to_string(),
                kind: ScaleKind::Price,
            },
        ])
    }
}

impl Setting for ScaleRules {
    const KEY: &'static str = "scale_barcode_rules";

    fn validate(&self) -> Result<(), AppError> {
        for rule in &self.0 {
            if !is_filled(&rule.name) {
                return Err(invalid("name", "scale_rule_name_missing"));
            }
            if !is_filled(&rule.pattern) {
                return Err(invalid("pattern", "scale_rule_pattern_missing"));
            }
        }
        Ok(())
    }
}

/// The prefix of barcodes the shop prints for its own items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct InternalBarcode {
    /// 2 or 3 digits starting with 2 (the GS1 in-store range 20–29). Example: "200".
    pub prefix: String,
}

impl Default for InternalBarcode {
    fn default() -> InternalBarcode {
        InternalBarcode {
            prefix: "200".to_string(),
        }
    }
}

impl Setting for InternalBarcode {
    const KEY: &'static str = "internal_barcode";

    fn validate(&self) -> Result<(), AppError> {
        let prefix = &self.prefix;
        let right_length = (2..=3).contains(&prefix.len());
        let all_digits = prefix.bytes().all(|byte| byte.is_ascii_digit());
        if !right_length || !all_digits || !prefix.starts_with('2') {
            return Err(invalid("prefix", "internal_prefix_invalid"));
        }
        Ok(())
    }
}

/// Where and how often backups are made (rule SET-11). `app` checks the folders can be written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct BackupSettings {
    /// Empty means the app data `backups/` folder, which `app` fills in.
    pub primary_dir: String,
    /// A second copy, for example on a USB drive.
    pub secondary_dir: Option<String>,
    /// Minutes between backups, 1 to 1440 (one day).
    pub interval_min: u32,
    /// How many backups to keep, at least 1.
    pub keep_last: u32,
}

impl Default for BackupSettings {
    fn default() -> BackupSettings {
        BackupSettings {
            primary_dir: String::new(),
            secondary_dir: None,
            interval_min: 60,
            keep_last: 50,
        }
    }
}

impl Setting for BackupSettings {
    const KEY: &'static str = "backup";

    fn validate(&self) -> Result<(), AppError> {
        if !(1..=1440).contains(&self.interval_min) {
            return Err(invalid("interval_min", "backup_interval_out_of_range"));
        }
        if self.keep_last == 0 {
            return Err(invalid("keep_last", "backup_keep_out_of_range"));
        }
        Ok(())
    }
}

/// Whether detailed (info and debug) logging is on. Errors are always logged (rule BAK-07).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(default)]
#[ts(export)]
pub struct LoggingSettings {
    pub detailed: bool,
}

impl Setting for LoggingSettings {
    const KEY: &'static str = "logging";

    fn validate(&self) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code_of(result: Result<(), AppError>) -> String {
        match result {
            Err(AppError::Validation { code, .. }) => code,
            other => format!("{other:?}"),
        }
    }

    #[test]
    fn set_10_printer_connections_need_their_details() {
        let mut hardware = HardwareSettings {
            receipt_printer: Some(PrinterConnection::Tcp {
                host: "192.168.1.50".to_string(),
                port: 0,
            }),
            ..HardwareSettings::default()
        };
        assert_eq!(
            code_of(hardware.validate()),
            "printer_connection_incomplete"
        );
        hardware.receipt_printer = Some(PrinterConnection::Preview);
        hardware.label_printer = Some(PrinterConnection::Cups {
            queue: String::new(),
        });
        assert_eq!(
            code_of(hardware.validate()),
            "printer_connection_incomplete"
        );
        hardware.label_printer = None;
        assert!(hardware.validate().is_ok());
    }

    #[test]
    fn set_10_scale_needs_a_port_and_a_profile() {
        let mut hardware = HardwareSettings {
            scale: Some(ScaleSettings::default()),
            ..HardwareSettings::default()
        };
        assert_eq!(code_of(hardware.validate()), "scale_port_missing");
        hardware.scale = Some(ScaleSettings {
            port: "COM3".to_string(),
            ..ScaleSettings::default()
        });
        assert_eq!(code_of(hardware.validate()), "scale_profile_missing");
    }

    #[test]
    fn set_10_scale_rules_need_a_name_and_pattern() {
        assert!(ScaleRules::default().validate().is_ok());
        let mut rules = ScaleRules::default();
        rules.0[0].name = String::new();
        assert_eq!(code_of(rules.validate()), "scale_rule_name_missing");
        let mut rules = ScaleRules::default();
        rules.0[1].pattern = " ".to_string();
        assert_eq!(code_of(rules.validate()), "scale_rule_pattern_missing");
    }

    #[test]
    fn internal_prefix_is_in_the_gs1_in_store_range() {
        for (prefix, ok) in [
            ("200", true),
            ("29", true),
            ("2", false),
            ("300", false),
            ("2a0", false),
            ("2000", false),
        ] {
            let barcode = InternalBarcode {
                prefix: prefix.to_string(),
            };
            assert_eq!(barcode.validate().is_ok(), ok, "{prefix}");
        }
    }

    #[test]
    fn set_11_backup_interval_and_count_rules() {
        assert!(BackupSettings::default().validate().is_ok());
        let every_two_days = BackupSettings {
            interval_min: 2880,
            ..BackupSettings::default()
        };
        assert_eq!(
            code_of(every_two_days.validate()),
            "backup_interval_out_of_range"
        );
        let keep_none = BackupSettings {
            keep_last: 0,
            ..BackupSettings::default()
        };
        assert_eq!(code_of(keep_none.validate()), "backup_keep_out_of_range");
    }

    #[test]
    fn set_10_printer_connection_is_sent_with_its_kind() {
        let json = serde_json::to_string(&PrinterConnection::Tcp {
            host: "printer".to_string(),
            port: 9100,
        })
        .unwrap_or_default();
        assert_eq!(json, r#"{"kind":"tcp","host":"printer","port":9100}"#);
    }
}
