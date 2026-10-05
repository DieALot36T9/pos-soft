//! A printer installed in Windows. Raw bytes go through the Windows print spooler:
//! OpenPrinter, StartDocPrinter with the "RAW" data type, WritePrinter.
//! The `escpos` crate has no spooler connection, so this file calls Windows itself
//! (Reuse Map). It is the only file in the crate allowed to use `unsafe`.
#![allow(unsafe_code)]

use std::time::Duration;

use windows::Win32::Graphics::Printing::{
    ClosePrinter, DOC_INFO_1W, EndDocPrinter, EndPagePrinter, EnumPrintersW, OpenPrinterW,
    PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL, PRINTER_HANDLE, PRINTER_INFO_4W,
    StartDocPrinterW, StartPagePrinter, WritePrinter,
};
use windows::core::{Error, HSTRING, PCWSTR, PWSTR};

use super::PrinterStatus;
use crate::wait::run_with_timeout;
use crate::{Device, HwError};

/// The spooler normally takes a job in well under a second. A stuck spooler mustn't hang the app.
const SPOOLER_TIMEOUT: Duration = Duration::from_secs(10);

/// Sends raw bytes to the Windows printer with this name.
pub(crate) fn send(printer_name: &str, bytes: &[u8]) -> Result<(), HwError> {
    let printer_name = printer_name.to_string();
    let bytes = bytes.to_vec();
    // The printer handle is opened, used and closed on the helper thread.
    run_with_timeout(Device::Printer, SPOOLER_TIMEOUT, move || {
        let handle = open(&printer_name)?;
        let result = write_raw_document(handle, &bytes);
        // SAFETY: `handle` came from OpenPrinterW and is closed exactly once, here.
        let _ = unsafe { ClosePrinter(handle) };
        result
    })
}

/// `Unknown` if Windows knows the printer (the spooler can't say if it's switched on),
/// `Offline` if it doesn't.
pub(crate) fn status(printer_name: &str) -> PrinterStatus {
    let printer_name = printer_name.to_string();
    let opened = run_with_timeout(Device::Printer, SPOOLER_TIMEOUT, move || {
        let handle = open(&printer_name)?;
        // SAFETY: `handle` came from OpenPrinterW and is closed exactly once, here.
        let _ = unsafe { ClosePrinter(handle) };
        Ok(())
    });
    match opened {
        Ok(()) => PrinterStatus::Unknown,
        Err(error) => PrinterStatus::Offline {
            detail: error.to_string(),
        },
    }
}

/// The names of every printer installed in Windows, including shared network printers.
pub(crate) fn installed_printers() -> Result<Vec<String>, HwError> {
    run_with_timeout(Device::Printer, SPOOLER_TIMEOUT, enumerate_printers)
}

fn open(printer_name: &str) -> Result<PRINTER_HANDLE, HwError> {
    let name = HSTRING::from(printer_name);
    let mut handle = PRINTER_HANDLE::default();
    // SAFETY: `name` lives until the call returns and `handle` is a valid place to write to.
    unsafe { OpenPrinterW(&name, &mut handle, None) }.map_err(|error| HwError::NotConnected {
        device: Device::Printer,
        detail: format!("Windows printer \"{printer_name}\": {}", error.message()),
    })?;
    Ok(handle)
}

fn write_raw_document(handle: PRINTER_HANDLE, bytes: &[u8]) -> Result<(), HwError> {
    let mut document_name = wide("POS raw job");
    // "RAW": the spooler passes the bytes to the printer unchanged.
    let mut data_type = wide("RAW");
    let document = DOC_INFO_1W {
        pDocName: PWSTR(document_name.as_mut_ptr()),
        pOutputFile: PWSTR::null(),
        pDatatype: PWSTR(data_type.as_mut_ptr()),
    };
    // SAFETY: `handle` is open and `document` points at strings that live until the call returns.
    let job = unsafe { StartDocPrinterW(handle, 1, &document) };
    if job == 0 {
        // A printer whose driver can't take raw data (e.g. "Microsoft Print to PDF") ends here.
        return Err(HwError::Refused {
            device: Device::Printer,
            detail: Error::from_thread().message(),
        });
    }

    let length = u32::try_from(bytes.len()).map_err(|_| HwError::Refused {
        device: Device::Printer,
        detail: "the job is larger than 4 GB".to_string(),
    })?;
    let mut written = 0u32;
    // SAFETY: `handle` is open with a started document; `bytes` holds `length` bytes.
    let page_started = unsafe { StartPagePrinter(handle) }.as_bool();
    let wrote_all = page_started
        && unsafe { WritePrinter(handle, bytes.as_ptr().cast(), length, &mut written) }.as_bool()
        && written == length;
    let failure = Error::from_thread().message();
    // SAFETY: ends the page and document started above on the same open handle.
    unsafe {
        let _ = EndPagePrinter(handle);
        let _ = EndDocPrinter(handle);
    }
    if wrote_all {
        Ok(())
    } else {
        Err(HwError::SendFailed {
            device: Device::Printer,
            detail: format!("wrote {written} of {length} bytes: {failure}"),
        })
    }
}

fn enumerate_printers() -> Result<Vec<String>, HwError> {
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    let mut needed = 0u32;
    let mut count = 0u32;
    // The first call only asks how much room the list needs, so its "buffer too small" error is expected.
    // SAFETY: no buffer is passed; Windows only writes the two counters.
    let _ = unsafe { EnumPrintersW(flags, PCWSTR::null(), 4, None, &mut needed, &mut count) };
    if needed == 0 {
        return Ok(Vec::new());
    }
    // u64 items keep the buffer aligned for the PRINTER_INFO_4W records Windows writes into it.
    let mut buffer = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: the byte view covers exactly the u64 buffer, which outlives it.
    let byte_view = unsafe {
        std::slice::from_raw_parts_mut(buffer.as_mut_ptr().cast::<u8>(), buffer.len() * 8)
    };
    // SAFETY: the buffer is at least `needed` bytes, as Windows asked.
    unsafe {
        EnumPrintersW(
            flags,
            PCWSTR::null(),
            4,
            Some(byte_view),
            &mut needed,
            &mut count,
        )
    }
    .map_err(|error| HwError::NotConnected {
        device: Device::Printer,
        detail: error.message(),
    })?;
    // SAFETY: Windows wrote `count` PRINTER_INFO_4W records at the start of the aligned buffer.
    let printers = unsafe {
        std::slice::from_raw_parts(buffer.as_ptr().cast::<PRINTER_INFO_4W>(), count as usize)
    };
    let mut names = Vec::new();
    for printer in printers {
        // SAFETY: each name points at a text inside the same buffer, ending with a zero.
        if let Ok(name) = unsafe { printer.pPrinterName.to_string() } {
            names.push(name);
        }
    }
    Ok(names)
}

/// A text as Windows wants it: UTF-16 with a zero at the end.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
