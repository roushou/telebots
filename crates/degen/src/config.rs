//! Typed application configuration, loaded and validated from the
//! environment once at startup.
//!
//! `main` calls [`Config::from_env`] and everything else consumes the typed fields.

use std::{env, fmt};

use anyhow::{Result, bail};

/// Env var names, defined once so docs, code, and `.env.example` stay in sync.
pub mod keys {
    /// Telegram bot token from @BotFather.
    pub const TELEGRAM_BOT_TOKEN: &str = "TELEBOTS_API_KEY_DEGEN";
    /// CoinMarketCap API key from pro.coinmarketcap.com.
    pub const COINMARKETCAP_API_KEY: &str = "COINMARKETCAP_API_KEY";
}

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets so `Config` can be logged safely.
#[derive(Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub coinmarketcap_api_key: String,
}

impl Config {
    /// Load from the process environment
    ///
    /// Reports every missing variable at once, each with a hint.
    pub fn from_env() -> Result<Self> {
        Self::load(&|k| env::var(k))
    }

    /// Load from an arbitrary reader so tests can pass a map instead of
    /// mutating the process-global environment (`env::set_var` is `unsafe`
    /// in edition 2024).
    fn load(read: &dyn Fn(&str) -> Result<String, env::VarError>) -> Result<Self> {
        let mut errors = Vec::new();

        let telegram_bot_token = required(
            read,
            &mut errors,
            keys::TELEGRAM_BOT_TOKEN,
            "get a token from @BotFather",
        );
        let coinmarketcap_api_key = required(
            read,
            &mut errors,
            keys::COINMARKETCAP_API_KEY,
            "get one at pro.coinmarketcap.com",
        );

        if !errors.is_empty() {
            bail!(
                "missing required environment variables:\n{}",
                errors
                    .iter()
                    .map(|e| format!("  - {e}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        Ok(Self {
            telegram_bot_token,
            coinmarketcap_api_key,
        })
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("telegram_bot_token", &"<redacted>")
            .field("coinmarketcap_api_key", &"<redacted>")
            .finish()
    }
}

/// Fetch a required var; empty counts as missing (`.env.example` ships empty
/// values, so an unfilled copy must fail loudly instead of passing `""`).
/// On failure, records an error and returns an unused placeholder — the
/// caller bails before using it.
fn required(
    read: &dyn Fn(&str) -> Result<String, env::VarError>,
    errors: &mut Vec<String>,
    key: &str,
    hint: &str,
) -> String {
    match read(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            errors.push(format!("{key} must be set — {hint}"));
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Load config from an in-memory map instead of the process env.
    fn load_with(vars: &[(&str, &str)]) -> Result<Config> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::load(&|k| map.get(k).cloned().ok_or(env::VarError::NotPresent))
    }

    #[test]
    fn loads_all_vars() {
        let cfg = load_with(&[
            (keys::TELEGRAM_BOT_TOKEN, "tok"),
            (keys::COINMARKETCAP_API_KEY, "key"),
        ])
        .unwrap();

        assert_eq!(cfg.telegram_bot_token, "tok");
        assert_eq!(cfg.coinmarketcap_api_key, "key");
    }

    #[test]
    fn reports_every_missing_var_at_once() {
        let err = load_with(&[]).unwrap_err();
        let msg = format!("{err:#}");
        for key in [keys::TELEGRAM_BOT_TOKEN, keys::COINMARKETCAP_API_KEY] {
            assert!(msg.contains(key), "error should mention {key}: {msg}");
        }
    }

    #[test]
    fn empty_values_count_as_missing() {
        let err = load_with(&[
            (keys::TELEGRAM_BOT_TOKEN, ""),
            (keys::COINMARKETCAP_API_KEY, "key"),
        ])
        .unwrap_err();
        assert!(format!("{err:#}").contains(keys::TELEGRAM_BOT_TOKEN));
    }

    #[test]
    fn debug_redacts_secrets() {
        let cfg = load_with(&[
            (keys::TELEGRAM_BOT_TOKEN, "super-secret-token"),
            (keys::COINMARKETCAP_API_KEY, "super-secret-key"),
        ])
        .unwrap();
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
    }
}
