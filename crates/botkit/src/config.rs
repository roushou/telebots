//! Typed secrets for config structs: redacted in `Debug`, transparent to
//! serde, and only reachable through explicit accessors.

use std::fmt;

/// A value that never leaks into `Debug` output — tokens, API keys, ...
///
/// `#[derive(Deserialize)]` on the containing struct treats `Secret<T>` as
/// `T` (via `#[serde(transparent)]`), so a config field can be declared
/// `Secret<String>` and still deserialize from the environment as a string.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wrap a value as a secret.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the secret value. This is the only way out; prefer
    /// [`Secret::into_inner`] to keep the secret owned and local.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consume the secret and return its value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

impl<T> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl From<Secret<String>> for String {
    fn from(secret: Secret<String>) -> Self {
        secret.0
    }
}

impl From<&str> for Secret<String> {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts() {
        let secret = Secret::new("hunter2".to_string());
        assert_eq!(format!("{secret:?}"), "<redacted>");
        assert_eq!(secret.expose(), "hunter2");
        assert_eq!(secret.into_inner(), "hunter2");
    }

    #[test]
    fn from_str_constructs_string_secret() {
        let secret: Secret<String> = "tok".into();
        assert_eq!(secret.expose(), "tok");
    }
}
