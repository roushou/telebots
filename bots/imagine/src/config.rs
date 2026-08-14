//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use anyhow::{Context, Result};
use botkit::Secret;
use serde::Deserialize;

/// Default database path used when `IMAGINE_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "imagine.db";

/// Default metrics port used when `TELEBOTS_METRICS_PORT` is unset.
pub const DEFAULT_METRICS_PORT: u16 = 9102;

/// Fully validated runtime configuration. Secrets render as `<redacted>` in
/// `Debug` output via [`Secret`].
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Telegram bot token from @BotFather (`TELEBOTS_TELEGRAM_API_KEY`).
    #[serde(rename = "telebots_telegram_api_key")]
    pub telegram_bot_token: Secret<String>,
    /// Cloudflare API token with Workers AI access (`CLOUDFLARE_API_TOKEN`).
    #[serde(rename = "cloudflare_api_token")]
    pub cloudflare_api_token: Secret<String>,
    /// Cloudflare account id (`CLOUDFLARE_ACCOUNT_ID`).
    #[serde(rename = "cloudflare_account_id")]
    pub cloudflare_account_id: Secret<String>,
    /// SQLite database path (`IMAGINE_DB_PATH`).
    #[serde(rename = "imagine_db_path", default = "default_db_path")]
    pub db_path: String,
    /// Metrics port the monitor polls (`TELEBOTS_METRICS_PORT`).
    #[serde(rename = "telebots_metrics_port", default = "default_metrics_port")]
    pub metrics_port: u16,
}

fn default_db_path() -> String {
    DEFAULT_DB_PATH.to_string()
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
            cloudflare_api_token: "super-secret-cf".into(),
            cloudflare_account_id: "super-secret-acct".into(),
            db_path: "imagine.db".into(),
            metrics_port: 9102,
        };
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
        // Non-secret fields stay visible.
        assert!(debug.contains("imagine.db"));
    }

    #[test]
    fn loads_from_environment() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("CLOUDFLARE_API_TOKEN", "cf-token"),
            ("CLOUDFLARE_ACCOUNT_ID", "acct"),
            ("IMAGINE_DB_PATH", "/data/imagine.db"),
            ("TELEBOTS_METRICS_PORT", "9102"),
        ]))?;
        assert_eq!(cfg.telegram_bot_token.expose(), "tok");
        assert_eq!(cfg.cloudflare_api_token.expose(), "cf-token");
        assert_eq!(cfg.cloudflare_account_id.expose(), "acct");
        assert_eq!(cfg.db_path, "/data/imagine.db");
        assert_eq!(cfg.metrics_port, 9102);
        Ok(())
    }

    #[test]
    fn defaults_db_path_and_metrics_port() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("CLOUDFLARE_API_TOKEN", "cf-token"),
            ("CLOUDFLARE_ACCOUNT_ID", "acct"),
        ]))?;
        assert_eq!(cfg.db_path, DEFAULT_DB_PATH);
        assert_eq!(cfg.metrics_port, DEFAULT_METRICS_PORT);
        Ok(())
    }
}
