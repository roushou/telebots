//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use std::fmt;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Default metrics port used when `TELEBOTS_METRICS_PORT` is unset.
pub const DEFAULT_METRICS_PORT: u16 = 9101;

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// Telegram bot token from @BotFather (`TELEBOTS_TELEGRAM_API_KEY`).
    #[serde(rename = "telebots_telegram_api_key")]
    pub telegram_bot_token: String,
    /// CoinMarketCap API key (`COINMARKETCAP_API_KEY`).
    #[serde(rename = "coinmarketcap_api_key")]
    pub coinmarketcap_api_key: String,
    /// Metrics port the monitor polls (`TELEBOTS_METRICS_PORT`).
    #[serde(rename = "telebots_metrics_port", default = "default_metrics_port")]
    pub metrics_port: u16,
}

fn default_metrics_port() -> u16 {
    DEFAULT_METRICS_PORT
}

impl Config {
    /// Load from the process environment (the caller loads `.env` first).
    pub fn from_env() -> Result<Self> {
        Self::from_source(config::Environment::default())
    }

    fn from_source(env: config::Environment) -> Result<Self> {
        config::Config::builder()
            .add_source(env.ignore_empty(true).try_parsing(true))
            .build()
            .context("failed to read the environment")?
            .try_deserialize()
            .context("invalid configuration")
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("telegram_bot_token", &"<redacted>")
            .field("coinmarketcap_api_key", &"<redacted>")
            .field("metrics_port", &self.metrics_port)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn env(vars: &[(&str, &str)]) -> config::Environment {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        config::Environment::default().source(Some(map))
    }

    #[test]
    fn debug_redacts_secrets() {
        let cfg = Config {
            telegram_bot_token: "super-secret-token".into(),
            coinmarketcap_api_key: "super-secret-key".into(),
            metrics_port: 9101,
        };
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
    }

    #[test]
    fn loads_from_environment() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("COINMARKETCAP_API_KEY", "key"),
            ("TELEBOTS_METRICS_PORT", "9101"),
        ]))?;
        assert_eq!(cfg.telegram_bot_token, "tok");
        assert_eq!(cfg.coinmarketcap_api_key, "key");
        assert_eq!(cfg.metrics_port, 9101);
        Ok(())
    }

    #[test]
    fn defaults_metrics_port() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("COINMARKETCAP_API_KEY", "key"),
        ]))?;
        assert_eq!(cfg.metrics_port, DEFAULT_METRICS_PORT);
        Ok(())
    }
}
