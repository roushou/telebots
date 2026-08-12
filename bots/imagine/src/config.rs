//! Typed application configuration, loaded and validated from the
//! environment once at startup. Loaded via dotenvy from the bot's own `.env`.

use std::{env, fmt};

use anyhow::{Result, bail};

/// Env var names, defined once so docs, code, and `.env.example` stay in sync.
pub mod keys {
    /// Telegram bot token from @BotFather.
    pub const TELEGRAM_BOT_TOKEN: &str = "TELEBOTS_TELEGRAM_API_KEY";
    /// Cloudflare API token with Workers AI access.
    pub const CLOUDFLARE_API_TOKEN: &str = "CLOUDFLARE_API_TOKEN";
    /// Cloudflare account id (the Workers AI REST endpoint needs it).
    pub const CLOUDFLARE_ACCOUNT_ID: &str = "CLOUDFLARE_ACCOUNT_ID";
    /// SQLite database path (default: `imagine.db` in the working dir).
    pub const DB_PATH: &str = "IMAGINE_DB_PATH";
}

/// Default database path used when `IMAGINE_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "imagine.db";

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets so `Config` can be logged safely.
#[derive(Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub cloudflare_api_token: String,
    pub cloudflare_account_id: String,
    pub db_path: String,
}

impl Config {
    /// Load from the process environment (via dotenvy). Reports every
    /// missing variable at once.
    pub fn from_env() -> Result<Self> {
        let mut errors = Vec::new();
        let telegram_bot_token = required(
            &mut errors,
            keys::TELEGRAM_BOT_TOKEN,
            "get a token from @BotFather",
        );
        let cloudflare_api_token = required(
            &mut errors,
            keys::CLOUDFLARE_API_TOKEN,
            "create one at dash.cloudflare.com with Workers AI permission",
        );
        let cloudflare_account_id = required(
            &mut errors,
            keys::CLOUDFLARE_ACCOUNT_ID,
            "find it in the Workers AI dashboard URL",
        );
        let db_path = optional(keys::DB_PATH).unwrap_or_else(|| DEFAULT_DB_PATH.to_string());

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
            cloudflare_api_token,
            cloudflare_account_id,
            db_path,
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
            .finish()
    }
}

fn required(errors: &mut Vec<String>, key: &str, hint: &str) -> String {
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            errors.push(format!("{key} must be set — {hint}"));
            String::new()
        }
    }
}

fn optional(key: &str) -> Option<String> {
    match env::var(key) {
        Ok(v) if !v.trim().is_empty() => Some(v),
        _ => None,
    }
}
