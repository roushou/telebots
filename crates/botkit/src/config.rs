//! Typed env-var config loading, shared by every bot.

use std::{collections::HashMap, env, fmt, path::Path};

use thiserror::Error;

/// Missing required configuration; the message lists every missing variable
/// with its hint.
#[derive(Debug, Error)]
#[error("missing required environment variables:\n{0}")]
pub struct ConfigError(pub String);

/// One environment variable a bot reads.
#[derive(Debug, Clone, Copy)]
pub struct Key {
    name: &'static str,
    required: bool,
    secret: bool,
    hint: &'static str,
    default: Option<&'static str>,
}

impl Key {
    /// A required, redacted variable (tokens, API keys).
    pub fn secret(name: &'static str, hint: &'static str) -> Self {
        Self {
            name,
            required: true,
            secret: true,
            hint,
            default: None,
        }
    }

    /// A required, non-secret variable.
    pub fn plain(name: &'static str, hint: &'static str) -> Self {
        Self {
            name,
            required: true,
            secret: false,
            hint,
            default: None,
        }
    }

    /// An optional variable; pair with [`Key::default`].
    pub fn optional(name: &'static str) -> Self {
        Self {
            name,
            required: false,
            secret: false,
            hint: "",
            default: None,
        }
    }

    /// The value used when the variable is unset.
    pub fn default(mut self, value: &'static str) -> Self {
        self.default = Some(value);
        self
    }
}

/// Configuration values loaded from the environment.
///
/// `Debug` redacts secret keys.
#[derive(Clone)]
pub struct Env {
    values: HashMap<&'static str, String>,
    secrets: Vec<&'static str>,
}

impl Env {
    /// Load a dotenv file into the process environment. A missing file is
    /// fine — env may come from compose `env_file` instead.
    pub fn load_file(path: impl AsRef<Path>) {
        let _ = dotenvy::from_path(path.as_ref());
    }

    /// Load `spec` from the process environment; every missing required
    /// variable is reported at once, each with its hint.
    pub fn load(spec: &[Key]) -> Result<Self, ConfigError> {
        Self::load_from(spec, &|k| env::var(k))
    }

    /// Load from an arbitrary reader so tests can pass a map instead of
    /// mutating the process-global environment.
    fn load_from(
        spec: &[Key],
        read: &dyn Fn(&str) -> Result<String, env::VarError>,
    ) -> Result<Self, ConfigError> {
        let mut values = HashMap::new();
        let mut errors = Vec::new();

        for key in spec {
            match read(key.name) {
                Ok(v) if !v.trim().is_empty() => {
                    values.insert(key.name, v);
                }
                Ok(_) | Err(_) if key.required => {
                    errors.push(format!("{} must be set — {}", key.name, key.hint));
                }
                _ => {
                    if let Some(default) = key.default {
                        values.insert(key.name, default.to_string());
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(ConfigError(errors.join("\n")));
        }

        Ok(Self {
            values,
            secrets: spec.iter().filter(|k| k.secret).map(|k| k.name).collect(),
        })
    }

    /// The value of `key`. Present for required and defaulted keys after a
    /// successful [`Env::load`]; panics on keys absent from the spec.
    pub fn require(&self, key: &'static str) -> String {
        self.values
            .get(key)
            .cloned()
            .unwrap_or_else(|| panic!("config key {key} missing from the spec"))
    }

    /// The raw value of `key`, if set.
    pub fn optional(&self, key: &'static str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

impl fmt::Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (name, value) in &self.values {
            if self.secrets.contains(name) {
                map.entry(name, &"<redacted>");
            } else {
                map.entry(name, value);
            }
        }
        map.finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn load_with(spec: &[Key], vars: &[(&str, &str)]) -> Result<Env, ConfigError> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Env::load_from(spec, &|k| {
            map.get(k).cloned().ok_or(env::VarError::NotPresent)
        })
    }

    fn token() -> Key {
        Key::secret("TOKEN", "get one from @BotFather")
    }

    #[test]
    fn loads_all_vars() {
        let env = load_with(&[token()], &[("TOKEN", "tok")]).unwrap();
        assert_eq!(env.require("TOKEN"), "tok");
    }

    #[test]
    fn reports_every_missing_var_at_once() {
        let spec = &[token(), Key::plain("KEY", "get one on the dashboard")];
        let err = load_with(spec, &[]).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("TOKEN"));
        assert!(msg.contains("KEY"));
    }

    #[test]
    fn empty_values_count_as_missing() {
        let err = load_with(&[token()], &[("TOKEN", "")]).unwrap_err();
        assert!(format!("{err:#}").contains("TOKEN"));
    }

    #[test]
    fn defaults_fill_optional_keys() {
        let key = Key::optional("DB").default("app.db");
        let env = load_with(&[key], &[]).unwrap();
        assert_eq!(env.require("DB"), "app.db");
    }

    #[test]
    fn optional_is_none_when_unset() {
        let env = load_with(&[Key::optional("DB")], &[]).unwrap();
        assert_eq!(env.optional("DB"), None);
    }

    #[test]
    fn debug_redacts_secrets() {
        let env = load_with(&[token()], &[("TOKEN", "super-secret")]).unwrap();
        let debug = format!("{env:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn debug_shows_plain_values() {
        let env = load_with(&[Key::plain("PORT", "the port")], &[("PORT", "8080")]).unwrap();
        let debug = format!("{env:?}");
        assert!(debug.contains("8080"));
        assert!(!debug.contains("<redacted>"));
    }
}
