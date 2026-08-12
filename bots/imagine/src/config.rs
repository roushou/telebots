//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use std::fmt;

use anyhow::{Context, Result};
use botkit::config::{Env, Key};

/// Env var names, defined once so docs, code, and `.env.example` stay in
/// sync.
pub mod keys {
    /// Telegram bot token from @BotFather.
    pub const TELEGRAM_BOT_TOKEN: &str = "TELEBOTS_TELEGRAM_API_KEY";
    /// Cloudflare API token with Workers AI access.
    pub const CLOUDFLARE_API_TOKEN: &str = "CLOUDFLARE_API_TOKEN";
    /// Cloudflare account id (the Workers AI REST endpoint needs it).
    pub const CLOUDFLARE_ACCOUNT_ID: &str = "CLOUDFLARE_ACCOUNT_ID";
    /// SQLite database path (default: `imagine.db` in the working dir).
    pub const DB_PATH: &str = "IMAGINE_DB_PATH";
    /// Metrics port the monitor polls.
    pub const METRICS_PORT: &str = "TELEBOTS_METRICS_PORT";
}

/// Default database path used when `IMAGINE_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "imagine.db";

/// Default metrics port used when `TELEBOTS_METRICS_PORT` is unset.
pub const DEFAULT_METRICS_PORT: &str = "9102";

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets.
#[derive(Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub cloudflare_api_token: String,
    pub cloudflare_account_id: String,
    pub db_path: String,
    pub metrics_port: u16,
}

impl Config {
    /// Load from the process environment (after `botkit::Env::load_file`).
    pub fn from_env() -> Result<Self> {
        let env = Env::load(&[
            Key::secret(keys::TELEGRAM_BOT_TOKEN, "get a token from @BotFather"),
            Key::secret(
                keys::CLOUDFLARE_API_TOKEN,
                "create one at dash.cloudflare.com with Workers AI permission",
            ),
            Key::plain(
                keys::CLOUDFLARE_ACCOUNT_ID,
                "find it in the Workers AI dashboard URL",
            ),
            Key::optional(keys::DB_PATH).default(DEFAULT_DB_PATH),
            Key::optional(keys::METRICS_PORT).default(DEFAULT_METRICS_PORT),
        ])?;
        Ok(Self {
            telegram_bot_token: env.require(keys::TELEGRAM_BOT_TOKEN),
            cloudflare_api_token: env.require(keys::CLOUDFLARE_API_TOKEN),
            cloudflare_account_id: env.require(keys::CLOUDFLARE_ACCOUNT_ID),
            db_path: env.require(keys::DB_PATH),
            metrics_port: env
                .require(keys::METRICS_PORT)
                .parse()
                .context("TELEBOTS_METRICS_PORT must be a number")?,
        })
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
