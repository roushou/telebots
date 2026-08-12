//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use std::fmt;

use anyhow::{Context, Result};
use serde::Deserialize;

/// Default database path used when `IMAGINE_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "imagine.db";

/// Default metrics port used when `TELEBOTS_METRICS_PORT` is unset.
pub const DEFAULT_METRICS_PORT: u16 = 9102;

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// Telegram bot token from @BotFather (`TELEBOTS_TELEGRAM_API_KEY`).
    #[serde(rename = "telebots_telegram_api_key")]
    pub telegram_bot_token: String,
    /// Cloudflare API token with Workers AI access (`CLOUDFLARE_API_TOKEN`).
    #[serde(rename = "cloudflare_api_token")]
    pub cloudflare_api_token: String,
    /// Cloudflare account id (`CLOUDFLARE_ACCOUNT_ID`).
    #[serde(rename = "cloudflare_account_id")]
    pub cloudflare_account_id: String,
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
        config::Config::builder()
            .add_source(
                config::Environment::default()
                    .ignore_empty(true)
                    .try_parsing(true),
            )
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
            .field("cloudflare_api_token", &"<redacted>")
            .field("cloudflare_account_id", &"<redacted>")
            .field("db_path", &self.db_path)
            .field("metrics_port", &self.metrics_port)
            .finish()
    }
}
