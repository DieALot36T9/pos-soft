//! Plain rules with no database: what a good PIN or password looks like,
//! and how the lockout counts failures (Security Model, USR-02).

use pos_core::Timestamp;

use crate::error::AuthError;

/// Wrong tries in a row before a user is locked (USR-02).
pub(crate) const MAX_FAILED_ATTEMPTS: u32 = 5;
/// How long a lock lasts: 60 seconds (USR-02).
pub(crate) const LOCK_MS: i64 = 60_000;

/// A PIN is 4 to 8 digits. Example: "1234".
pub(crate) fn check_pin(pin: &str) -> Result<(), AuthError> {
    let length = pin.chars().count();
    let all_digits = pin.chars().all(|character| character.is_ascii_digit());
    if (4..=8).contains(&length) && all_digits {
        Ok(())
    } else {
        Err(AuthError::PinFormat)
    }
}

/// A password has at least 8 characters.
pub(crate) fn check_password(password: &str) -> Result<(), AuthError> {
    if password.chars().count() >= 8 {
        Ok(())
    } else {
        Err(AuthError::PasswordTooShort)
    }
}

/// A user's lockout state, as stored in `auth_users`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LockState {
    pub failed_attempts: u32,
    pub locked_until: Option<Timestamp>,
}

impl LockState {
    /// No failures and no lock. A correct secret puts the user back here.
    pub(crate) const CLEAR: LockState = LockState {
        failed_attempts: 0,
        locked_until: None,
    };
}

/// The end of the lock, if the user is still locked at `now`.
pub(crate) fn locked_until(state: LockState, now: Timestamp) -> Option<Timestamp> {
    state.locked_until.filter(|until| now < *until)
}

/// The state after one more wrong try. The fifth wrong try locks the user for 60 s
/// and starts the count again, so after the lock the user gets five new tries.
pub(crate) fn after_failure(state: LockState, now: Timestamp) -> LockState {
    let failed_attempts = state.failed_attempts + 1;
    if failed_attempts >= MAX_FAILED_ATTEMPTS {
        LockState {
            failed_attempts: 0,
            locked_until: Some(Timestamp(now.0 + LOCK_MS)),
        }
    } else {
        LockState {
            failed_attempts,
            locked_until: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: Timestamp = Timestamp(1_000_000);

    #[test]
    fn usr_06_pins_are_4_to_8_digits() {
        assert!(check_pin("1234").is_ok());
        assert!(check_pin("12345678").is_ok());
        assert!(check_pin("123").is_err());
        assert!(check_pin("123456789").is_err());
        assert!(check_pin("12a4").is_err());
        assert!(check_pin("١٢٣٤").is_err()); // digits from another script
    }

    #[test]
    fn usr_06_passwords_have_at_least_8_characters() {
        assert!(check_password("8 chars!").is_ok());
        assert!(check_password("7 chars").is_err());
    }

    #[test]
    fn usr_02_fifth_failure_locks_for_60_seconds() {
        let mut state = LockState::CLEAR;
        for _ in 0..4 {
            state = after_failure(state, NOW);
            assert_eq!(locked_until(state, NOW), None);
        }
        state = after_failure(state, NOW);
        assert_eq!(locked_until(state, NOW), Some(Timestamp(NOW.0 + 60_000)));
        assert_eq!(state.failed_attempts, 0);
    }

    #[test]
    fn usr_02_lock_is_over_at_exactly_60_seconds() {
        let state = LockState {
            failed_attempts: 0,
            locked_until: Some(Timestamp(NOW.0 + 60_000)),
        };
        assert!(locked_until(state, Timestamp(NOW.0 + 59_999)).is_some());
        assert!(locked_until(state, Timestamp(NOW.0 + 60_000)).is_none());
    }
}
