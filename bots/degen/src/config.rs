//! Typed application configuration, loaded and validated from the
//! environment once at startup.

use std::fmt;

use anyhow::Result;
use botkit::config::{Env, Key};

/// Env var names, defined once so docs, code, and `.env.example` stay in
/// sync.
pub mod keys {
    /// Telegram bot token from @BotFather.
    pub const TELEGRAM_BOT_TOKEN: &str = "TELEBOTS_TELEGRAM_API_KEY";
    /// CoinMarketCap API key from pro.coinmarketcap.com.
    pub const COINMARKETCAP_API_KEY: &str = "COINMARKETCAP_API_KEY";
}

/// Fully validated runtime configuration.
///
/// `Debug` redacts secrets.
#[derive(Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub coinmarketcap_api_key: String,
}

impl Config {
    /// Load from the process environment (after `botkit::Env::load_file`).
    pub fn from_env() -> Result<Self> {
        let env = Env::load(&[
            Key::secret(keys::TELEGRAM_BOT_TOKEN, "get a token from @BotFather"),
            Key::secret(
                keys::COINMARKETCAP_API_KEY,
                "get one at pro.coinmarketcap.com",
            ),
        ])?;
        Ok(Self {
            telegram_bot_token: env.require(keys::TELEGRAM_BOT_TOKEN),
            coinmarketcap_api_key: env.require(keys::COINMARKETCAP_API_KEY),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_secrets() {
        let cfg = Config {
            telegram_bot_token: "super-secret-token".into(),
            coinmarketcap_api_key: "super-secret-key".into(),
        };
        let debug = format!("{cfg:?}");
        assert!(!debug.contains("super-secret"), "leaked secrets: {debug}");
        assert!(debug.contains("<redacted>"));
    }
}
