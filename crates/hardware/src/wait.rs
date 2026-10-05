//! Helpers that put a time limit on device calls, so a stuck device never hangs the app.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use crate::{Device, HwError};

/// Runs `work` on its own thread and waits at most `limit` for it.
/// If the time runs out, the thread is left to finish on its own and the caller gets `Timeout`.
pub(crate) fn run_with_timeout<T: Send + 'static>(
    device: Device,
    limit: Duration,
    work: impl FnOnce() -> Result<T, HwError> + Send + 'static,
) -> Result<T, HwError> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        // The caller may have stopped waiting. Then nobody needs the result.
        let _ = sender.send(work());
    });
    match receiver.recv_timeout(limit) {
        Ok(result) => result,
        Err(_) => Err(timeout(device, limit)),
    }
}

/// What a finished program printed, and whether it said it worked.
pub(crate) struct ProgramOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs a program (e.g. `lp`), feeds it `input`, and stops it if it takes longer than `limit`.
pub(crate) fn run_program(
    device: Device,
    program: &str,
    args: &[&str],
    input: &[u8],
    limit: Duration,
) -> Result<ProgramOutput, HwError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| HwError::NotConnected {
            device,
            detail: format!("could not start {program}: {error}"),
        })?;

    // Writing and reading happen on their own threads, so a full pipe can't block us.
    let stdin = child.stdin.take();
    let input = input.to_vec();
    let writer = thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            // A program that quits early closes its input. Its exit code tells us what happened.
            let _ = stdin.write_all(&input);
        }
    });
    let stdout_reader = read_all_later(child.stdout.take());
    let stderr_reader = read_all_later(child.stderr.take());

    let deadline = Instant::now() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(timeout(device, limit));
            }
            Err(error) => {
                return Err(HwError::SendFailed {
                    device,
                    detail: format!("{program}: {error}"),
                });
            }
        }
    };
    let _ = writer.join();
    Ok(ProgramOutput {
        success: status.success(),
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
    })
}

/// Reads a program's output to the end on a separate thread.
fn read_all_later(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        text
    })
}

/// The `Timeout` error for a wait of `limit`.
pub(crate) fn timeout(device: Device, limit: Duration) -> HwError {
    HwError::Timeout {
        device,
        waited_ms: u64::try_from(limit.as_millis()).unwrap_or(u64::MAX),
    }
}

/// Locks a mutex. If another thread panicked while holding it, the data is still used:
/// a fake device or a list of jobs is never left half-changed by our code.
pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hw_08_slow_work_ends_in_a_timeout() {
        let result = run_with_timeout(Device::Printer, Duration::from_millis(20), || {
            thread::sleep(Duration::from_millis(500));
            Ok(())
        });
        assert_eq!(
            result,
            Err(HwError::Timeout {
                device: Device::Printer,
                waited_ms: 20
            })
        );
    }

    #[test]
    fn quick_work_returns_its_result() {
        let result = run_with_timeout(Device::Printer, Duration::from_secs(5), || Ok(7));
        assert_eq!(result, Ok(7));
    }

    #[test]
    fn a_missing_program_is_not_connected() {
        let result = run_program(
            Device::Printer,
            "no-such-program-for-the-pos-test",
            &[],
            b"",
            Duration::from_secs(1),
        );
        assert!(matches!(result, Err(HwError::NotConnected { .. })));
    }

    #[cfg(unix)]
    #[test]
    fn hw_08_a_program_that_hangs_is_stopped() {
        let started = Instant::now();
        let result = run_program(
            Device::Printer,
            "sleep",
            &["5"],
            b"",
            Duration::from_millis(100),
        );
        assert!(matches!(result, Err(HwError::Timeout { .. })));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn a_program_gets_its_input_and_returns_its_output() {
        let output = run_program(
            Device::Printer,
            "cat",
            &[],
            b"hello",
            Duration::from_secs(5),
        )
        .unwrap();
        assert!(output.success);
        assert_eq!(output.stdout, "hello");
    }
}
