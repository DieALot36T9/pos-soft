//! Hashing and checking passwords, PINs and recovery codes with argon2id.
//! Settings follow the OWASP Password Storage Cheat Sheet (Reuse Map):
//! 19 MiB memory, 2 iterations, 1 lane.

use argon2::password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::AuthError;

/// Memory for one hash, in KiB: 19 MiB.
const MEMORY_KIB: u32 = 19 * 1024;
/// How many passes over the memory.
const ITERATIONS: u32 = 2;
/// How many lanes work side by side.
const LANES: u32 = 1;

/// The hasher with our settings.
fn hasher() -> Result<Argon2<'static>, AuthError> {
    let params = Params::new(MEMORY_KIB, ITERATIONS, LANES, None)
        .map_err(|error| AuthError::Hashing(error.to_string()))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Turns a secret into a salted hash to store. Example: "$argon2id$v=19$m=19456,t=2,p=1$…".
pub(crate) fn hash_secret(plain: &str) -> Result<String, AuthError> {
    let hash = hasher()?
        .hash_password(plain.as_bytes())
        .map_err(|error| AuthError::Hashing(error.to_string()))?;
    Ok(hash.to_string())
}

/// Checks a typed secret against a stored hash. A damaged hash counts as "no match",
/// so a broken row can't let anyone in.
pub(crate) fn secret_matches(plain: &str, stored_hash: &str) -> Result<bool, AuthError> {
    let Ok(parsed) = PasswordHash::new(stored_hash) else {
        return Ok(false);
    };
    // The settings stored in the hash are used, so old hashes keep working
    // if the settings change later.
    Ok(hasher()?.verify_password(plain.as_bytes(), &parsed).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_matches_its_secret_only() {
        let hash = hash_secret("1234").unwrap();
        assert!(secret_matches("1234", &hash).unwrap());
        assert!(!secret_matches("1235", &hash).unwrap());
    }

    #[test]
    fn the_same_secret_gets_a_different_salt_each_time() {
        assert_ne!(hash_secret("1234").unwrap(), hash_secret("1234").unwrap());
    }

    #[test]
    fn a_damaged_hash_never_matches() {
        assert!(!secret_matches("1234", "not a hash").unwrap());
    }
}
