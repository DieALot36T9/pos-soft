//! The recovery code for a forgotten admin password (USR-09).
//! The shop keeps the code on paper. Only its argon2id hash is stored.

use pos_core::{Id, Secret, Timestamp};
use rand::RngExt;
use rusqlite::{OptionalExtension, Transaction, named_params};

use crate::error::AuthError;
use crate::hashing::{hash_secret, secret_matches};
use crate::roles::ADMIN_ROLE_ID;
use crate::rules::check_password;
use crate::users::{SecretKind, load_user, set_secret};

/// Letters and digits that are hard to misread: no 0/O, 1/I/L or U.
const CODE_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";
/// 20 characters from 30 choices: about 98 bits, far too many to guess.
const CODE_LENGTH: usize = 20;
/// The code is shown in groups of 4 so it is easy to copy. Example: "7KQ2-M9XD-4RTW-PZ8C-H3NB".
const GROUP_SIZE: usize = 4;

/// Makes a new recovery code and replaces the old one. Returns the plain code,
/// which `app` shows once and never again.
pub fn create_recovery_code(
    transaction: &Transaction,
    now: Timestamp,
) -> Result<Secret<String>, AuthError> {
    let code = new_code();
    let hash = hash_secret(&normalize(&code))?;
    transaction.execute(
        "INSERT INTO auth_recovery (id, code_hash, created_at) VALUES (1, :hash, :now)
         ON CONFLICT (id) DO UPDATE SET code_hash = :hash, created_at = :now",
        named_params! { ":hash": hash, ":now": now.0 },
    )?;
    Ok(Secret::new(code))
}

/// Sets a new password for an admin when the recovery code is right. The old code
/// stops working and a new one is returned, to show once (USR-09).
pub fn reset_with_recovery(
    transaction: &Transaction,
    code: Secret<String>,
    admin: &Id,
    new_password: Secret<String>,
    now: Timestamp,
) -> Result<Secret<String>, AuthError> {
    let stored: Option<String> = transaction
        .query_row(
            "SELECT code_hash FROM auth_recovery WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let Some(stored) = stored else {
        return Err(AuthError::RecoveryCodeWrong);
    };
    if !secret_matches(&normalize(code.expose()), &stored)? {
        return Err(AuthError::RecoveryCodeWrong);
    }
    let user = load_user(transaction, admin)?;
    if user.role_id != ADMIN_ROLE_ID {
        return Err(AuthError::RecoveryUserNotAdmin);
    }
    if !user.active {
        return Err(AuthError::UserInactive);
    }
    check_password(new_password.expose())?;
    set_secret(transaction, admin, SecretKind::Password, new_password, now)?;
    create_recovery_code(transaction, now)
}

/// A fresh random code with dashes between the groups.
fn new_code() -> String {
    let mut random = rand::rng();
    let mut code = String::new();
    for position in 0..CODE_LENGTH {
        if position > 0 && position % GROUP_SIZE == 0 {
            code.push('-');
        }
        let index = random.random_range(0..CODE_ALPHABET.len());
        code.push(char::from(CODE_ALPHABET[index]));
    }
    code
}

/// The code as it is hashed: capitals only, no dashes or spaces.
/// So "7kq2 m9xd…" and "7KQ2-M9XD-…" are the same code.
fn normalize(typed: &str) -> String {
    typed
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .map(|character| character.to_ascii_uppercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usr_09_codes_use_only_easy_to_read_characters() {
        let code = new_code();
        assert_eq!(code.len(), 24);
        for group in code.split('-') {
            assert_eq!(group.len(), GROUP_SIZE);
            assert!(group.bytes().all(|byte| CODE_ALPHABET.contains(&byte)));
        }
    }

    #[test]
    fn usr_09_typed_codes_ignore_dashes_spaces_and_small_letters() {
        assert_eq!(normalize("7kq2 m9xd-4RTW"), "7KQ2M9XD4RTW");
    }
}
