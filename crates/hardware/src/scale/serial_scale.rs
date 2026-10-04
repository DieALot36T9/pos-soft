//! A real scale on a serial port, and the loop that reads weights from it.

use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

use super::parse::{LineReader, LineSplitter, StabilityCheck};
use super::{DataBits, Parity, Reading, Scale, ScaleMode, ScaleProfile, SerialSettings, StopBits};
use crate::wait::timeout;
use crate::{Device, HwError};

/// How long one wait for bytes may take, so the loop can check the overall time limit often.
const POLL_WAIT: Duration = Duration::from_millis(50);
/// A request scale that hasn't answered after this long is asked again.
const REQUEST_WAIT: Duration = Duration::from_millis(500);
/// How much of the scale's raw data to keep for `NoWeight` errors.
const RAW_KEPT: usize = 512;

/// The two things the read loop needs from a connection: send bytes, receive bytes.
/// The real serial port and the recorded test data both provide them.
pub(crate) trait ScaleLink {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()>;
    /// Waits a short time for bytes. Returns 0 if none came.
    fn receive(&mut self, buffer: &mut [u8]) -> io::Result<usize>;
}

impl ScaleLink for Box<dyn serialport::SerialPort> {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.write_all(bytes)?;
        self.flush()
    }

    fn receive(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.read(buffer) {
            // The port's own timeout (POLL_WAIT) ran out with nothing received.
            Err(error) if error.kind() == io::ErrorKind::TimedOut => Ok(0),
            other => other,
        }
    }
}

pub(crate) struct SerialScale {
    settings: SerialSettings,
    profile: ScaleProfile,
}

impl SerialScale {
    pub(crate) fn new(settings: SerialSettings, profile: ScaleProfile) -> SerialScale {
        SerialScale { settings, profile }
    }

    fn open(&self) -> Result<Box<dyn serialport::SerialPort>, HwError> {
        let data_bits = match self.settings.data_bits {
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        };
        let parity = match self.settings.parity {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        };
        let stop_bits = match self.settings.stop_bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        };
        serialport::new(&self.settings.port, self.settings.baud)
            .data_bits(data_bits)
            .parity(parity)
            .stop_bits(stop_bits)
            .timeout(POLL_WAIT)
            .open()
            .map_err(|error| HwError::NotConnected {
                device: Device::Scale,
                detail: format!("{}: {error}", self.settings.port),
            })
    }
}

impl Scale for SerialScale {
    fn read(&self, timeout: Duration) -> Result<Reading, HwError> {
        // Check the profile before opening the port, so a wrong pattern is reported as such.
        let reader = LineReader::new(&self.profile)?;
        let mut port = self.open()?;
        read_weight(&mut port, &self.profile, &reader, timeout)
    }
}

/// Reads lines until the weight is stable or `limit` runs out.
/// Request scales are asked again after each answer, or after `REQUEST_WAIT` with no answer.
pub(crate) fn read_weight(
    link: &mut dyn ScaleLink,
    profile: &ScaleProfile,
    reader: &LineReader,
    limit: Duration,
) -> Result<Reading, HwError> {
    let deadline = Instant::now() + limit;
    let mut splitter = LineSplitter::default();
    let mut stability = StabilityCheck::new(profile.stable_repeats);
    let mut last_reading: Option<Reading> = None;
    let mut raw_seen = String::new();
    let mut buffer = [0u8; 256];
    let mut ask_now = true;
    let mut asked_at = Instant::now();

    while Instant::now() < deadline {
        if let ScaleMode::Request { command } = &profile.mode
            && (ask_now || asked_at.elapsed() >= REQUEST_WAIT)
        {
            link.send(command).map_err(|error| send_failed(&error))?;
            ask_now = false;
            asked_at = Instant::now();
        }
        let count = link
            .receive(&mut buffer)
            .map_err(|error| send_failed(&error))?;
        for line in splitter.push(&buffer[..count]) {
            ask_now = true;
            keep_raw(&mut raw_seen, &line);
            let Some(mut reading) = reader.read_line(&line) else {
                continue;
            };
            if !reader.has_stable_marker() {
                reading.stable = stability.add(&reading);
            }
            if reading.stable {
                return Ok(reading);
            }
            last_reading = Some(reading);
        }
    }

    match last_reading {
        // The weight never settled: the caller shows it as "moving".
        Some(reading) => Ok(reading),
        None if !raw_seen.is_empty() => Err(HwError::NoWeight { raw: raw_seen }),
        None => Err(timeout(Device::Scale, limit)),
    }
}

/// Keeps the most recent raw lines, up to `RAW_KEPT` bytes, for troubleshooting.
fn keep_raw(raw_seen: &mut String, line: &str) {
    if !raw_seen.is_empty() {
        raw_seen.push('\n');
    }
    raw_seen.push_str(line);
    while raw_seen.len() > RAW_KEPT {
        match raw_seen.find('\n') {
            Some(end_of_first_line) => {
                raw_seen.drain(..=end_of_first_line);
            }
            // Only one line is left. Lines are short (see LineSplitter), so keep it whole.
            None => break,
        }
    }
}

fn send_failed(error: &io::Error) -> HwError {
    HwError::SendFailed {
        device: Device::Scale,
        detail: error.to_string(),
    }
}
