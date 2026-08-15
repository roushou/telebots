//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use anyhow::{Context, Result};
use botkit::Secret;
use serde::Deserialize;

/// Default database path used when `BUD_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "bud.db";

/// Default metrics port used when `TELEBOTS_METRICS_PORT` is unset.
pub const DEFAULT_METRICS_PORT: u16 = 9103;

/// Default number of prior messages kept in the conversation context.
pub const DEFAULT_MAX_HISTORY: usize = 20;

/// The default system prompt (persona) used when none is set per chat.
pub fn default_system_prompt() -> String {
    "You are bud, a friendly and helpful assistant living inside Telegram. \
     Answer helpfully and concisely, prefer plain text over heavy markdown, \
     and be direct when you are unsure about something."
        .to_string()
}

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
    /// SQLite database path (`BUD_DB_PATH`).
    #[serde(rename = "bud_db_path", default = "default_db_path")]
    pub db_path: String,
    /// Default system prompt (`BUD_SYSTEM_PROMPT`).
    #[serde(rename = "bud_system_prompt", default = "default_system_prompt")]
    pub system_prompt: String,
    /// How many prior messages to keep in context (`BUD_MAX_HISTORY`).
    #[serde(rename = "bud_max_history", default = "default_max_history")]
    pub max_history: usize,
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

fn default_max_history() -> usize {
    DEFAULT_MAX_HISTORY
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
            db_path: "bud.db".into(),
            system_prompt: "be nice".into(),
            max_history: 20,
            metrics_port: 9103,
        };
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
        // Non-secret fields stay visible.
        assert!(debug.contains("bud.db"));
    }

    #[test]
    fn loads_from_environment() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("CLOUDFLARE_API_TOKEN", "cf-token"),
            ("CLOUDFLARE_ACCOUNT_ID", "acct"),
            ("BUD_DB_PATH", "/data/bud.db"),
            ("BUD_SYSTEM_PROMPT", "be terse"),
            ("BUD_MAX_HISTORY", "5"),
            ("TELEBOTS_METRICS_PORT", "9103"),
        ]))?;
        assert_eq!(cfg.telegram_bot_token.expose(), "tok");
        assert_eq!(cfg.cloudflare_api_token.expose(), "cf-token");
        assert_eq!(cfg.cloudflare_account_id.expose(), "acct");
        assert_eq!(cfg.db_path, "/data/bud.db");
        assert_eq!(cfg.system_prompt, "be terse");
        assert_eq!(cfg.max_history, 5);
        assert_eq!(cfg.metrics_port, 9103);
        Ok(())
    }

    #[test]
    fn defaults_apply_when_unset() -> Result<()> {
        let cfg = Config::from_source(env(&[
            ("TELEBOTS_TELEGRAM_API_KEY", "tok"),
            ("CLOUDFLARE_API_TOKEN", "cf-token"),
            ("CLOUDFLARE_ACCOUNT_ID", "acct"),
        ]))?;
        assert_eq!(cfg.db_path, DEFAULT_DB_PATH);
        assert_eq!(cfg.max_history, DEFAULT_MAX_HISTORY);
        assert_eq!(cfg.metrics_port, DEFAULT_METRICS_PORT);
        assert!(cfg.system_prompt.contains("bud"));
        Ok(())
    }
}
