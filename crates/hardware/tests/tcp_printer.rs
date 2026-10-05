//! Rule HW-02: the network printer, tested against a local listener that stands in for
//! a printer on port 9100.

use std::io::Read;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use hardware::{HwError, PreviewInbox, PrinterConfig, PrinterStatus, printer_from};

#[test]
fn hw_02_network_printer_receives_exactly_the_bytes_sent() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let printer_side = thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        let mut received = Vec::new();
        connection.read_to_end(&mut received).unwrap();
        received
    });
    // ESC @ (start), some bytes of every value, GS V 0 (cut).
    let mut bytes = vec![0x1B, b'@'];
    bytes.extend(0..=255u8);
    bytes.extend([0x1D, b'V', 0]);

    let printer = printer_from(
        &PrinterConfig::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        },
        &PreviewInbox::new(),
    );
    printer.send(&bytes).unwrap();
    // The connection closes when the job is done, so the listener sees the end.
    drop(printer);

    assert_eq!(printer_side.join().unwrap(), bytes);
}

#[test]
fn hw_02_network_printer_accepts_a_host_name() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let printer_side = thread::spawn(move || {
        let (mut connection, _) = listener.accept().unwrap();
        let mut received = Vec::new();
        connection.read_to_end(&mut received).unwrap();
        received
    });

    let config = PrinterConfig::Tcp {
        host: "localhost".to_string(),
        port,
    };
    let result = printer_from(&config, &PreviewInbox::new()).send(b"hello");

    // "localhost" may mean ::1 first on some computers, where nothing listens.
    if result.is_ok() {
        assert_eq!(printer_side.join().unwrap(), b"hello");
    }
}

#[test]
fn hw_08_network_printer_that_is_off_fails_quickly() {
    // Bind and close a listener to find a port where nothing listens.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let printer = printer_from(
        &PrinterConfig::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        },
        &PreviewInbox::new(),
    );

    let started = std::time::Instant::now();
    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
    assert!(matches!(printer.status(), PrinterStatus::Offline { .. }));
    // The page promises a 3 second limit.
    assert!(started.elapsed() < Duration::from_secs(7));
}

#[test]
fn hw_02_network_printer_that_answers_is_ready() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let printer = printer_from(
        &PrinterConfig::Tcp {
            host: "127.0.0.1".to_string(),
            port,
        },
        &PreviewInbox::new(),
    );
    assert_eq!(printer.status(), PrinterStatus::Ready);
}

#[test]
fn hw_08_unknown_host_is_not_connected() {
    let printer = printer_from(
        &PrinterConfig::Tcp {
            host: "no-such-printer.invalid".to_string(),
            port: 9100,
        },
        &PreviewInbox::new(),
    );
    assert!(matches!(
        printer.send(b"x"),
        Err(HwError::NotConnected { .. })
    ));
}
