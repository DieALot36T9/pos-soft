//! Finding the weight in what a scale sends. Plain functions, no device, so they are easy to test.

use regex::Regex;

use super::{Reading, ScaleMode, ScaleProfile, WeightUnit};
use crate::{Device, HwError};

/// A line longer than this without an end-of-line is not a weight line. It is handed on
/// as it is, so "Read scale" can show what the scale is sending.
const LONGEST_LINE: usize = 256;

/// Checks that a profile can work: the pattern is a valid regular expression with a
/// `value` group, and a request profile has a command to send.
pub fn check_profile(profile: &ScaleProfile) -> Result<(), HwError> {
    LineReader::new(profile).map(|_| ())
}

/// Reads weights from lines with one profile.
pub(crate) struct LineReader {
    pattern: Regex,
    default_unit: WeightUnit,
    stable_marker: Option<String>,
}

impl LineReader {
    pub(crate) fn new(profile: &ScaleProfile) -> Result<LineReader, HwError> {
        let bad_settings = |detail: String| HwError::BadSettings {
            device: Device::Scale,
            detail,
        };
        let pattern = Regex::new(&profile.pattern)
            .map_err(|error| bad_settings(format!("the scale pattern is wrong: {error}")))?;
        if !pattern.capture_names().any(|name| name == Some("value")) {
            return Err(bad_settings(
                "the scale pattern has no group named \"value\"".to_string(),
            ));
        }
        if let ScaleMode::Request { command } = &profile.mode
            && command.is_empty()
        {
            return Err(bad_settings(
                "the scale request command is empty".to_string(),
            ));
        }
        Ok(LineReader {
            pattern,
            default_unit: profile.default_unit,
            stable_marker: profile
                .stable_marker
                .clone()
                .filter(|marker| !marker.is_empty()),
        })
    }

    /// Finds the weight in one line. `None` if the line has no weight, e.g. a status line.
    /// `stable` is set only from the stable marker; `StabilityCheck` handles scales without one.
    pub(crate) fn read_line(&self, line: &str) -> Option<Reading> {
        let found = self.pattern.captures(line)?;
        let value_milli = parse_milli(found.name("value")?.as_str())?;
        let unit = match found.name("unit") {
            Some(unit) => parse_unit(unit.as_str())?,
            None => self.default_unit,
        };
        let stable = match &self.stable_marker {
            Some(marker) => line.contains(marker.as_str()),
            None => false,
        };
        Some(Reading {
            value_milli,
            unit,
            stable,
            raw: line.to_string(),
        })
    }

    pub(crate) fn has_stable_marker(&self) -> bool {
        self.stable_marker.is_some()
    }
}

/// Turns a number like "+  1.234" or "-0.5" into thousandths: 1234, -500.
/// Spaces after the sign are allowed (CAS scales send them). More than 3 decimals is refused.
pub(crate) fn parse_milli(text: &str) -> Option<i64> {
    let text = text.trim();
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, text[1..].trim_start()),
        b'+' => (false, text[1..].trim_start()),
        _ => (false, text),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) || fraction.len() > 3 {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    // "5" → 500 thousandths of the "0.5": pad the fraction to three digits.
    let fraction: i64 = format!("{fraction:0<3}").parse().ok()?;
    let value = whole.checked_mul(1000)?.checked_add(fraction)?;
    Some(if negative { -value } else { value })
}

fn parse_unit(text: &str) -> Option<WeightUnit> {
    match text.trim().to_ascii_lowercase().as_str() {
        "g" => Some(WeightUnit::G),
        "kg" => Some(WeightUnit::Kg),
        "lb" | "lbs" => Some(WeightUnit::Lb),
        _ => None,
    }
}

/// For scales with no stable marker: the weight is stable once it stays the same for
/// `repeats` readings in a row (rule HW-05).
pub(crate) struct StabilityCheck {
    repeats: u8,
    last: Option<(i64, WeightUnit)>,
    same_count: u8,
}

impl StabilityCheck {
    pub(crate) fn new(repeats: u8) -> StabilityCheck {
        StabilityCheck {
            repeats: repeats.max(1),
            last: None,
            same_count: 0,
        }
    }

    /// Counts this reading and says whether the weight is now stable.
    pub(crate) fn add(&mut self, reading: &Reading) -> bool {
        let current = (reading.value_milli, reading.unit);
        if self.last == Some(current) {
            self.same_count = self.same_count.saturating_add(1);
        } else {
            self.last = Some(current);
            self.same_count = 1;
        }
        self.same_count >= self.repeats
    }
}

/// Cuts the bytes from a scale into lines. Lines end at CR or LF. Other control bytes
/// (e.g. STX at the start of a Toledo answer) are dropped. Empty lines are skipped.
#[derive(Default)]
pub(crate) struct LineSplitter {
    pending: Vec<u8>,
}

impl LineSplitter {
    /// Adds received bytes and returns every line that is now complete.
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for byte in bytes {
            match *byte {
                b'\r' | b'\n' => self.finish_line(&mut lines),
                byte if byte.is_ascii_control() => {}
                byte => {
                    self.pending.push(byte);
                    if self.pending.len() >= LONGEST_LINE {
                        self.finish_line(&mut lines);
                    }
                }
            }
        }
        lines
    }

    fn finish_line(&mut self, lines: &mut Vec<String>) {
        if !self.pending.is_empty() {
            lines.push(String::from_utf8_lossy(&self.pending).into_owned());
            self.pending.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(pattern: &str) -> ScaleProfile {
        ScaleProfile {
            mode: ScaleMode::Continuous,
            pattern: pattern.to_string(),
            default_unit: WeightUnit::Kg,
            stable_marker: None,
            stable_repeats: 3,
        }
    }

    #[test]
    fn hw_05_numbers_become_thousandths_without_floats() {
        assert_eq!(parse_milli("1.234"), Some(1234));
        assert_eq!(parse_milli("+  1.234"), Some(1234));
        assert_eq!(parse_milli("-0.5"), Some(-500));
        assert_eq!(parse_milli("01.20"), Some(1200));
        assert_eq!(parse_milli("250"), Some(250_000));
        assert_eq!(parse_milli(" 0.000 "), Some(0));
    }

    #[test]
    fn hw_05_numbers_that_are_not_weights_are_refused() {
        assert_eq!(parse_milli(""), None);
        assert_eq!(parse_milli("+"), None);
        assert_eq!(parse_milli(".5"), None);
        assert_eq!(parse_milli("1.2345"), None);
        assert_eq!(parse_milli("1,234"), None);
        assert_eq!(parse_milli("99999999999999999999"), None);
    }

    #[test]
    fn hw_05_unit_comes_from_the_line_or_the_profile() {
        let reader =
            LineReader::new(&profile(r"(?i)(?P<value>\d+\.\d+)\s*(?P<unit>kg|g|lb)?")).unwrap();
        assert_eq!(reader.read_line("250.000 g").unwrap().unit, WeightUnit::G);
        assert_eq!(reader.read_line("2.205 LB").unwrap().unit, WeightUnit::Lb);
        assert_eq!(reader.read_line("1.000").unwrap().unit, WeightUnit::Kg);
        assert_eq!(reader.read_line("no weight here"), None);
    }

    #[test]
    fn hw_05_stable_marker_marks_the_reading() {
        let mut cas = profile(r"(?P<value>[-+]?\s*\d+\.\d+)(?P<unit>kg)");
        cas.stable_marker = Some("ST,".to_string());
        let reader = LineReader::new(&cas).unwrap();
        assert!(reader.read_line("ST,GS,+  1.234kg").unwrap().stable);
        assert!(!reader.read_line("US,GS,+  1.234kg").unwrap().stable);
    }

    #[test]
    fn hw_05_without_a_marker_three_equal_readings_are_stable() {
        let reading = |value_milli| Reading {
            value_milli,
            unit: WeightUnit::Kg,
            stable: false,
            raw: String::new(),
        };
        let mut check = StabilityCheck::new(3);
        assert!(!check.add(&reading(1234)));
        assert!(!check.add(&reading(1234)));
        assert!(!check.add(&reading(1236)));
        assert!(!check.add(&reading(1236)));
        assert!(check.add(&reading(1236)));
    }

    #[test]
    fn hw_05_bad_profiles_are_refused() {
        assert!(matches!(
            check_profile(&profile("(unclosed")),
            Err(HwError::BadSettings { .. })
        ));
        assert!(matches!(
            check_profile(&profile(r"\d+\.\d+")),
            Err(HwError::BadSettings { .. })
        ));
        let mut empty_request = profile(r"(?P<value>\d+)");
        empty_request.mode = ScaleMode::Request {
            command: Vec::new(),
        };
        assert!(matches!(
            check_profile(&empty_request),
            Err(HwError::BadSettings { .. })
        ));
    }

    #[test]
    fn lines_are_cut_at_cr_or_lf_and_control_bytes_dropped() {
        let mut splitter = LineSplitter::default();
        assert_eq!(splitter.push(b"\x0201.2"), Vec::<String>::new());
        assert_eq!(splitter.push(b"34\r\n\x02?A\r"), vec!["01.234", "?A"]);
    }

    #[test]
    fn a_very_long_line_without_an_end_is_handed_on() {
        let mut splitter = LineSplitter::default();
        let lines = splitter.push(&[b'x'; LONGEST_LINE]);
        assert_eq!(lines.len(), 1);
    }
}
