//! Pretend scales. They behave like real ones: answers take time, a moving weight stays
//! "moving" until the time limit, and an unplugged scale times out.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{Reading, Scale, WeightUnit};
use crate::wait::{lock, timeout};
use crate::{Device, HwError};

/// A real scale at 9600 baud sends about ten lines a second.
const FAKE_LINE_GAP: Duration = Duration::from_millis(100);

/// A scale for tests that gives back a fixed list of answers, one per `read`.
pub struct MockScale {
    answers: Mutex<VecDeque<Result<Reading, HwError>>>,
    delay: Duration,
}

impl MockScale {
    /// A scale that answers each `read` with the next item of `answers`, at once.
    /// When the list is used up, it acts like an unplugged scale and times out.
    pub fn new(answers: Vec<Result<Reading, HwError>>) -> MockScale {
        MockScale {
            answers: Mutex::new(answers.into()),
            delay: Duration::ZERO,
        }
    }

    /// The same scale, but each answer takes this long. A `read` with a shorter time
    /// limit times out, like a real slow scale.
    pub fn with_delay(mut self, delay: Duration) -> MockScale {
        self.delay = delay;
        self
    }
}

impl Scale for MockScale {
    fn read(&self, limit: Duration) -> Result<Reading, HwError> {
        if self.delay > limit {
            thread::sleep(limit);
            return Err(timeout(Device::Scale, limit));
        }
        thread::sleep(self.delay);
        match lock(&self.answers).pop_front() {
            Some(answer) => answer,
            None => {
                thread::sleep(limit.saturating_sub(self.delay));
                Err(timeout(Device::Scale, limit))
            }
        }
    }
}

/// What the fake scale panel has set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FakeScaleState {
    /// The weight in thousandths of `unit`.
    pub value_milli: i64,
    pub unit: WeightUnit,
    /// False while the weight is "moving".
    pub stable: bool,
    /// False makes the scale act unplugged: every read times out.
    pub connected: bool,
}

/// The scale behind the dev-only "Fake scale" panel (rule HW-10). The panel and the
/// hardware thread each hold a copy; copies share the same state.
#[derive(Debug, Clone)]
pub struct FakeScale {
    state: Arc<Mutex<FakeScaleState>>,
}

impl FakeScale {
    /// A plugged-in fake scale showing a stable 0.000 kg.
    pub fn new() -> FakeScale {
        FakeScale {
            state: Arc::new(Mutex::new(FakeScaleState {
                value_milli: 0,
                unit: WeightUnit::Kg,
                stable: true,
                connected: true,
            })),
        }
    }

    /// Puts a weight on the scale. Example: `set_weight(1234, WeightUnit::Kg)` is 1.234 kg.
    pub fn set_weight(&self, value_milli: i64, unit: WeightUnit) {
        let mut state = lock(&self.state);
        state.value_milli = value_milli;
        state.unit = unit;
    }

    /// Switches the weight between settled (true) and moving (false).
    pub fn set_stable(&self, stable: bool) {
        lock(&self.state).stable = stable;
    }

    /// Plugs the scale in (true) or pulls it out (false).
    pub fn set_connected(&self, connected: bool) {
        lock(&self.state).connected = connected;
    }

    /// What the panel has set right now.
    pub fn state(&self) -> FakeScaleState {
        lock(&self.state).clone()
    }
}

impl Default for FakeScale {
    fn default() -> FakeScale {
        FakeScale::new()
    }
}

impl Scale for FakeScale {
    fn read(&self, limit: Duration) -> Result<Reading, HwError> {
        let deadline = Instant::now() + limit;
        let mut last_reading = None;
        // Like a real scale: a new line every so often, until the weight settles or time runs out.
        loop {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            thread::sleep(FAKE_LINE_GAP.min(deadline - now));
            let state = self.state();
            if !state.connected {
                continue;
            }
            let reading = Reading {
                value_milli: state.value_milli,
                unit: state.unit,
                stable: state.stable,
                raw: fake_line(&state),
            };
            if reading.stable {
                return Ok(reading);
            }
            last_reading = Some(reading);
        }
        last_reading.ok_or_else(|| timeout(Device::Scale, limit))
    }
}

/// The line a generic scale would send for this state, e.g. "ST  1.234 kg" or "US -0.050 kg".
fn fake_line(state: &FakeScaleState) -> String {
    let sign = if state.value_milli < 0 { "-" } else { " " };
    let whole = state.value_milli.unsigned_abs() / 1000;
    let thousandths = state.value_milli.unsigned_abs() % 1000;
    let unit = match state.unit {
        WeightUnit::G => "g",
        WeightUnit::Kg => "kg",
        WeightUnit::Lb => "lb",
    };
    let flag = if state.stable { "ST" } else { "US" };
    format!("{flag} {sign}{whole}.{thousandths:03} {unit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_line_looks_like_a_scale_line() {
        let mut state = FakeScale::new().state();
        state.value_milli = -50;
        state.stable = false;
        assert_eq!(fake_line(&state), "US -0.050 kg");
        state.value_milli = 1234;
        state.stable = true;
        assert_eq!(fake_line(&state), "ST  1.234 kg");
    }
}
