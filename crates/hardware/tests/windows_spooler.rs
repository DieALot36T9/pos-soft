//! Rule HW-02: printers installed in Windows. Run by hand on a Windows PC with
//! `cargo nextest run -p hardware --run-ignored only`. The full checklist with a real
//! receipt printer is part of M13 (first real hardware).
#![cfg(windows)]

use hardware::{HwError, PreviewInbox, PrinterConfig, list_printers, printer_from};

#[test]
#[ignore = "needs the Microsoft Print to PDF printer; run by hand on Windows"]
fn hw_02_print_to_pdf_refuses_raw_data() {
    let name = "Microsoft Print to PDF".to_string();
    assert!(list_printers().contains(&name), "printer not installed");
    let printer = printer_from(
        &PrinterConfig::WindowsSpooler { printer_name: name },
        &PreviewInbox::new(),
    );

    // This printer's driver can't take raw ESC/POS, so Windows refuses the job.
    let result = printer.send(b"\x1b@hello\n");

    assert!(
        matches!(
            result,
            Err(HwError::Refused { .. } | HwError::SendFailed { .. })
        ),
        "got {result:?}"
    );
}

#[test]
fn hw_02_unknown_windows_printer_is_not_connected() {
    let printer = printer_from(
        &PrinterConfig::WindowsSpooler {
            printer_name: "No such printer for the POS test".to_string(),
        },
        &PreviewInbox::new(),
    );
    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
}
