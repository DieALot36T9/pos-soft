//! Rule HW-05: each built-in profile reads the lines its scale sends (`tests/fixtures/`),
//! through the same read loop a real serial scale uses.

use std::collections::VecDeque;
use std::io;
use std::thread;
use std::time::Duration;

use super::parse::LineReader;
use super::serial_scale::{ScaleLink, read_weight};
use super::{ScaleProfile, WeightUnit, builtin_scale_profiles};
use crate::{Device, HwError};

const GENERIC_LINES: &[u8] = include_bytes!("../../tests/fixtures/scale_generic_continuous.txt");
const CAS_LINES: &[u8] = include_bytes!("../../tests/fixtures/scale_cas.txt");
/// One answer per line; each answer ends with CR, as the scale sends it.
const TOLEDO_ANSWERS: &[u8] = include_bytes!("../../tests/fixtures/scale_toledo.bin");

/// Plays back recorded bytes like a serial port would: a few bytes at a time,
/// and nothing (after a short wait) when there's no more.
struct RecordedLink {
    incoming: VecDeque<Vec<u8>>,
    /// For request scales: one answer is queued each time the app sends something.
    answers: VecDeque<Vec<u8>>,
    sent: Vec<Vec<u8>>,
}

impl RecordedLink {
    fn continuous(bytes: &[u8]) -> RecordedLink {
        RecordedLink {
            // Odd-sized pieces, so lines arrive split across reads like on a real port.
            incoming: bytes.chunks(5).map(<[u8]>::to_vec).collect(),
            answers: VecDeque::new(),
            sent: Vec::new(),
        }
    }

    fn answering(answers: &[u8]) -> RecordedLink {
        RecordedLink {
            incoming: VecDeque::new(),
            answers: answers
                .split(|byte| *byte == b'\n')
                .filter(|answer| !answer.is_empty())
                .map(<[u8]>::to_vec)
                .collect(),
            sent: Vec::new(),
        }
    }
}

impl ScaleLink for RecordedLink {
    fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.sent.push(bytes.to_vec());
        if let Some(answer) = self.answers.pop_front() {
            self.incoming.push_back(answer);
        }
        Ok(())
    }

    fn receive(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        match self.incoming.pop_front() {
            Some(chunk) => {
                buffer[..chunk.len()].copy_from_slice(&chunk);
                Ok(chunk.len())
            }
            None => {
                thread::sleep(Duration::from_millis(5));
                Ok(0)
            }
        }
    }
}

fn profile(id: &str) -> ScaleProfile {
    builtin_scale_profiles()
        .into_iter()
        .find(|(profile_id, _)| profile_id == id)
        .map(|(_, profile)| profile)
        .unwrap()
}

fn read(link: &mut RecordedLink, profile: &ScaleProfile) -> Result<super::Reading, HwError> {
    let reader = LineReader::new(profile).unwrap();
    read_weight(link, profile, &reader, Duration::from_millis(300))
}

#[test]
fn hw_05_generic_continuous_is_stable_after_three_equal_readings() {
    let mut link = RecordedLink::continuous(GENERIC_LINES);

    let reading = read(&mut link, &profile("generic_continuous")).unwrap();

    assert_eq!(reading.value_milli, 1234);
    assert_eq!(reading.unit, WeightUnit::Kg);
    assert!(reading.stable);
    assert_eq!(reading.raw, "   1.234 kg");
    // The fourth 1.234 line was never needed.
    assert!(!link.incoming.is_empty());
}

#[test]
fn hw_05_cas_waits_for_the_stable_marker() {
    let mut link = RecordedLink::continuous(CAS_LINES);

    let reading = read(&mut link, &profile("cas")).unwrap();

    assert_eq!(reading.value_milli, 1234);
    assert_eq!(reading.unit, WeightUnit::Kg);
    assert!(reading.stable);
    assert_eq!(reading.raw, "ST,GS,+  1.234kg");
}

#[test]
fn hw_05_toledo_asks_again_until_the_weight_settles() {
    let mut link = RecordedLink::answering(TOLEDO_ANSWERS);

    let reading = read(&mut link, &profile("toledo")).unwrap();

    assert_eq!(reading.value_milli, 1234);
    assert!(reading.stable);
    assert_eq!(reading.raw, "01.234");
    assert_eq!(link.sent, vec![b"W".to_vec(), b"W".to_vec(), b"W".to_vec()]);
}

#[test]
fn hw_05_weight_that_never_settles_comes_back_as_moving() {
    // Only the three unstable CAS lines.
    let unstable: Vec<u8> = CAS_LINES
        .split_inclusive(|byte| *byte == b'\n')
        .take(3)
        .flatten()
        .copied()
        .collect();
    let mut link = RecordedLink::continuous(&unstable);

    let reading = read(&mut link, &profile("cas")).unwrap();

    assert_eq!(reading.value_milli, 1236);
    assert!(!reading.stable);
}

#[test]
fn hw_05_data_without_a_weight_is_returned_raw_for_troubleshooting() {
    let mut link = RecordedLink::continuous(b"ERR 07\r\nHELLO\r\n");

    let result = read(&mut link, &profile("generic_continuous"));

    assert_eq!(
        result,
        Err(HwError::NoWeight {
            raw: "ERR 07\nHELLO".to_string()
        })
    );
}

#[test]
fn hw_08_a_silent_scale_times_out() {
    let mut link = RecordedLink::continuous(b"");

    let result = read(&mut link, &profile("generic_continuous"));

    assert_eq!(
        result,
        Err(HwError::Timeout {
            device: Device::Scale,
            waited_ms: 300
        })
    );
}

#[test]
fn hw_05_a_request_scale_that_does_not_answer_is_asked_again() {
    let mut link = RecordedLink::answering(b"");
    let toledo = profile("toledo");
    let reader = LineReader::new(&toledo).unwrap();

    let result = read_weight(&mut link, &toledo, &reader, Duration::from_millis(1200));

    assert!(matches!(result, Err(HwError::Timeout { .. })));
    // Asked at the start, then again after each 500 ms without an answer.
    assert_eq!(link.sent.len(), 3);
}
