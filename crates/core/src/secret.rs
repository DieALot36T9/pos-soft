//! A wrapper that keeps passwords, PINs and tokens out of logs (Security Model).

use std::fmt;

use serde::Deserialize;

/// Holds a secret such as a password. Printing it for a log shows `***`.
/// It can be read from the UI, but never sent back. Call `expose` to use the value.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps a secret value.
    pub fn new(value: T) -> Secret<T> {
        Secret(value)
    }

    /// The secret value itself. Use it only where it's needed, such as checking a password.
    pub fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("***")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_hides_the_value() {
        let pin = Secret::new("1234".to_string());
        assert_eq!(format!("{pin:?}"), "***");
        assert_eq!(pin.expose(), "1234");
    }

    #[test]
    fn can_be_read_from_the_ui() {
        let password: Secret<String> = serde_json::from_str(r#""hunter22""#).unwrap();
        assert_eq!(password.expose(), "hunter22");
    }
}
