//! Contract tests: the public API behaves as the module page says.
//! `docs/05 Modules/Module - hardware.md`.

use std::time::{Duration, Instant};

use hardware::{
    Device, FakeScale, HwError, MockPrinter, MockScale, PreviewInbox, PrinterConfig, PrinterStatus,
    RawPrinter, Reading, Scale, ScaleMode, ScaleProfile, SerialSettings, WeightUnit,
    builtin_scale_profiles, check_profile, list_printers, list_serial_ports, printer_from,
};
use pos_core::{AppError, Id};

fn kg(value_milli: i64, stable: bool) -> Reading {
    Reading {
        value_milli,
        unit: WeightUnit::Kg,
        stable,
        raw: String::new(),
    }
}

// ---------- Printers ----------

#[test]
fn hw_08_mock_printer_keeps_jobs_and_fails_when_told() {
    let printer = MockPrinter::new();
    printer.fail_next(HwError::Timeout {
        device: Device::Printer,
        waited_ms: 3000,
    });

    assert!(matches!(
        printer.send(b"first"),
        Err(HwError::Timeout { .. })
    ));
    printer.send(b"second").unwrap();

    // The failed job printed nothing; the next one printed.
    assert_eq!(printer.sent_jobs(), vec![b"second".to_vec()]);
}

#[test]
fn hw_08_mock_printer_is_as_slow_as_told() {
    let printer = MockPrinter::new().with_delay(Duration::from_millis(50));
    let started = Instant::now();
    printer.send(b"x").unwrap();
    assert!(started.elapsed() >= Duration::from_millis(50));
}

#[test]
fn hw_08_mock_printer_reports_the_status_it_was_given() {
    let printer = MockPrinter::new();
    assert_eq!(printer.status(), PrinterStatus::Ready);
    printer.set_status(PrinterStatus::Offline {
        detail: "paper out".to_string(),
    });
    assert!(matches!(printer.status(), PrinterStatus::Offline { .. }));
}

#[test]
fn hw_10_preview_printer_hands_its_jobs_to_the_inbox() {
    let inbox = PreviewInbox::new();
    let printer = printer_from(&PrinterConfig::Preview, &inbox);
    // GS v 0: one byte wide, two rows. Then ESC p: drawer pulse on pin 2.
    let bytes = [
        0x1D, b'v', b'0', 0, 1, 0, 2, 0, 0xF0, 0x0F, 0x1B, b'p', 0, 25, 250,
    ];

    printer.send(&bytes).unwrap();

    let jobs = inbox.take_jobs();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].pictures.len(), 1);
    assert_eq!(jobs[0].pictures[0].width, 8);
    assert_eq!(jobs[0].pictures[0].bits, vec![0xF0, 0x0F]);
    assert!(jobs[0].drawer_opened);
    assert_eq!(printer.status(), PrinterStatus::Ready);
}

#[test]
fn hw_02_file_printer_adds_each_job_to_the_file() {
    let path = std::env::temp_dir().join(format!("hardware-test-{}.bin", Id::new().as_str()));
    let printer = printer_from(
        &PrinterConfig::File { path: path.clone() },
        &PreviewInbox::new(),
    );

    printer.send(b"\x1b@one").unwrap();
    printer.send(b"two").unwrap();

    assert_eq!(std::fs::read(&path).unwrap(), b"\x1b@onetwo");
    assert_eq!(printer.status(), PrinterStatus::Ready);
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn hw_08_file_printer_in_a_missing_folder_fails_with_an_error() {
    let path = std::env::temp_dir()
        .join(format!("no-such-folder-{}", Id::new().as_str()))
        .join("out.bin");
    let printer = printer_from(&PrinterConfig::File { path }, &PreviewInbox::new());

    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
    assert!(matches!(printer.status(), PrinterStatus::Offline { .. }));
}

#[test]
fn hw_08_serial_printer_on_a_missing_port_fails_with_an_error() {
    let config = PrinterConfig::Serial {
        port: "/dev/no-such-port-for-the-pos-test".to_string(),
        baud: 9600,
    };
    let printer = printer_from(&config, &PreviewInbox::new());

    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
    assert!(matches!(printer.status(), PrinterStatus::Offline { .. }));
}

#[cfg(not(windows))]
#[test]
fn hw_02_windows_printer_fails_clearly_on_other_systems() {
    let config = PrinterConfig::WindowsSpooler {
        printer_name: "EPSON TM-T20".to_string(),
    };
    let printer = printer_from(&config, &PreviewInbox::new());

    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
    assert!(matches!(printer.status(), PrinterStatus::Offline { .. }));
}

#[cfg(windows)]
#[test]
fn hw_02_cups_printer_fails_clearly_on_windows() {
    let config = PrinterConfig::Cups {
        queue: "receipts".to_string(),
    };
    let printer = printer_from(&config, &PreviewInbox::new());
    assert!(printer.send(b"x").is_err());
}

#[test]
fn hw_02_listing_printers_and_ports_never_fails() {
    // The lists depend on the computer. They must come back quickly, even when empty.
    let started = Instant::now();
    let _printers = list_printers();
    let ports = list_serial_ports();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert!(ports.windows(2).all(|pair| pair[0].name <= pair[1].name));
}

#[test]
fn hw_02_printer_settings_are_saved_as_tagged_json() {
    // The settings module stores this JSON, so its shape is part of the contract.
    let config = PrinterConfig::Tcp {
        host: "192.168.1.50".to_string(),
        port: 9100,
    };
    let json = serde_json::to_string(&config).unwrap();
    assert_eq!(json, r#"{"kind":"tcp","host":"192.168.1.50","port":9100}"#);
    assert_eq!(
        serde_json::to_string(&PrinterConfig::Preview).unwrap(),
        r#"{"kind":"preview"}"#
    );
    let back: PrinterConfig =
        serde_json::from_str(r#"{"kind":"windows_spooler","printer_name":"POS-80"}"#).unwrap();
    assert_eq!(
        back,
        PrinterConfig::WindowsSpooler {
            printer_name: "POS-80".to_string()
        }
    );
}

// ---------- Scales ----------

#[test]
fn hw_05_three_profiles_ship_and_all_are_valid() {
    let profiles = builtin_scale_profiles();
    let ids: Vec<&str> = profiles.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["generic_continuous", "cas", "toledo"]);
    for (id, profile) in &profiles {
        assert_eq!(check_profile(profile), Ok(()), "profile {id}");
    }
}

#[test]
fn hw_05_a_profile_without_a_value_group_is_refused() {
    let profile = ScaleProfile {
        mode: ScaleMode::Continuous,
        pattern: r"\d+\.\d+".to_string(),
        default_unit: WeightUnit::Kg,
        stable_marker: None,
        stable_repeats: 3,
    };
    assert!(matches!(
        check_profile(&profile),
        Err(HwError::BadSettings {
            device: Device::Scale,
            ..
        })
    ));
}

#[test]
fn hw_05_scale_on_a_missing_port_fails_with_an_error() {
    let port = SerialSettings {
        port: "/dev/no-such-port-for-the-pos-test".to_string(),
        baud: 9600,
        data_bits: hardware::DataBits::Eight,
        parity: hardware::Parity::None,
        stop_bits: hardware::StopBits::One,
    };
    let (_, profile) = builtin_scale_profiles().remove(0);
    let scale = hardware::scale_from(&port, &profile);

    assert!(matches!(
        scale.read(Duration::from_millis(200)),
        Err(HwError::NotConnected {
            device: Device::Scale,
            ..
        })
    ));
}

#[test]
fn hw_05_scale_settings_are_saved_as_json() {
    let profile = ScaleProfile {
        mode: ScaleMode::Request {
            command: vec![0x05],
        },
        pattern: r"(?P<value>\d+)".to_string(),
        default_unit: WeightUnit::G,
        stable_marker: None,
        stable_repeats: 3,
    };
    let json = serde_json::to_string(&profile).unwrap();
    assert_eq!(
        json,
        r#"{"mode":{"kind":"request","command":[5]},"pattern":"(?P<value>\\d+)","default_unit":"g","stable_marker":null,"stable_repeats":3}"#
    );
}

#[test]
fn hw_08_mock_scale_answers_in_order_then_times_out() {
    let scale = MockScale::new(vec![
        Ok(kg(1180, false)),
        Ok(kg(1234, true)),
        Err(HwError::NoWeight {
            raw: "ERR".to_string(),
        }),
    ]);
    let limit = Duration::from_millis(50);

    assert_eq!(scale.read(limit), Ok(kg(1180, false)));
    assert_eq!(scale.read(limit), Ok(kg(1234, true)));
    assert!(matches!(scale.read(limit), Err(HwError::NoWeight { .. })));
    assert!(matches!(scale.read(limit), Err(HwError::Timeout { .. })));
}

#[test]
fn hw_08_slow_mock_scale_times_out_like_a_real_one() {
    let scale = MockScale::new(vec![Ok(kg(1234, true))]).with_delay(Duration::from_millis(200));
    assert!(matches!(
        scale.read(Duration::from_millis(20)),
        Err(HwError::Timeout {
            device: Device::Scale,
            waited_ms: 20
        })
    ));
}

#[test]
fn hw_10_fake_scale_gives_the_stable_weight_set_on_the_panel() {
    let scale = FakeScale::new();
    let panel = scale.clone();
    panel.set_weight(1234, WeightUnit::Kg);

    let reading = scale.read(Duration::from_secs(2)).unwrap();

    assert_eq!(reading.value_milli, 1234);
    assert_eq!(reading.unit, WeightUnit::Kg);
    assert!(reading.stable);
    assert_eq!(reading.raw, "ST  1.234 kg");
}

#[test]
fn hw_10_fake_scale_that_is_moving_returns_the_weight_as_moving_after_the_wait() {
    let scale = FakeScale::new();
    scale.set_weight(500, WeightUnit::Kg);
    scale.set_stable(false);
    let started = Instant::now();

    let reading = scale.read(Duration::from_millis(300)).unwrap();

    assert!(!reading.stable);
    assert_eq!(reading.value_milli, 500);
    assert!(started.elapsed() >= Duration::from_millis(300));
}

#[test]
fn hw_10_fake_scale_that_is_unplugged_times_out() {
    let scale = FakeScale::new();
    scale.set_connected(false);
    assert!(matches!(
        scale.read(Duration::from_millis(250)),
        Err(HwError::Timeout {
            device: Device::Scale,
            ..
        })
    ));
}

// ---------- Errors ----------

#[test]
fn hw_08_errors_reach_the_ui_as_hardware_errors_with_a_code() {
    let error: AppError = HwError::Timeout {
        device: Device::Printer,
        waited_ms: 3000,
    }
    .into();
    match error {
        AppError::Hardware {
            device,
            code,
            detail,
        } => {
            assert_eq!(device, "printer");
            assert_eq!(code, "timeout");
            assert!(detail.contains("3000 ms"));
        }
        other => panic!("expected a hardware error, got {other:?}"),
    }

    let no_weight: AppError = HwError::NoWeight {
        raw: "ERR 07".to_string(),
    }
    .into();
    assert!(matches!(
        no_weight,
        AppError::Hardware { ref device, ref code, ref detail }
            if device == "scale" && code == "no_weight" && detail.contains("ERR 07")
    ));
}

#[test]
fn hw_08_every_error_has_its_own_code() {
    let printer = Device::Printer;
    let detail = || "x".to_string();
    let codes: Vec<&str> = [
        HwError::Timeout {
            device: printer,
            waited_ms: 1,
        },
        HwError::NotConnected {
            device: printer,
            detail: detail(),
        },
        HwError::SendFailed {
            device: printer,
            detail: detail(),
        },
        HwError::Refused {
            device: printer,
            detail: detail(),
        },
        HwError::NoWeight { raw: detail() },
        HwError::BadSettings {
            device: printer,
            detail: detail(),
        },
    ]
    .iter()
    .map(HwError::code)
    .collect();
    assert_eq!(
        codes,
        vec![
            "timeout",
            "not_connected",
            "send_failed",
            "refused",
            "no_weight",
            "bad_settings"
        ]
    );
}
