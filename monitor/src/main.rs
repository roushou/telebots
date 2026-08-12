//! The admin monitor: polls every bot's `/metrics` endpoint, keeps status
//! snapshots in SQLite, and serves the dashboard + JSON API.

mod api;
mod db;
mod poller;

use anyhow::{Context, Result};
use botkit::config::{Env, Key};

/// One bot to watch: its display name and `/metrics` URL.
#[derive(Debug, Clone)]
pub struct BotTarget {
    pub name: String,
    pub url: String,
}

/// Env var names, defined once so docs, code, and `.env.example` stay in
/// sync.
pub mod keys {
    /// Comma-separated `name=url` pairs, e.g.
    /// `degen=http://degen:9101/metrics`.
    pub const BOTS: &str = "MONITOR_BOTS";
    /// SQLite path (default `monitor.db`).
    pub const DB_PATH: &str = "MONITOR_DB_PATH";
    /// HTTP port (default 9110).
    pub const PORT: &str = "MONITOR_PORT";
}

/// Default database path used when `MONITOR_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "monitor.db";

/// Default port used when `MONITOR_PORT` is unset.
pub const DEFAULT_PORT: &str = "9110";

/// Fully validated runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub bots: Vec<BotTarget>,
    pub db_path: String,
    pub port: u16,
}

impl Config {
    /// Load from the process environment (after `botkit::Env::load_file`).
    pub fn from_env() -> Result<Self> {
        let env = Env::load(&[
            Key::plain(keys::BOTS, "comma-separated name=url pairs"),
            Key::optional(keys::DB_PATH).default(DEFAULT_DB_PATH),
            Key::optional(keys::PORT).default(DEFAULT_PORT),
        ])?;
        let bots = Self::parse_targets(&env.require(keys::BOTS));
        if bots.is_empty() {
            anyhow::bail!("MONITOR_BOTS has no valid name=url entries");
        }
        Ok(Self {
            bots,
            db_path: env.require(keys::DB_PATH),
            port: env
                .require(keys::PORT)
                .parse()
                .context("MONITOR_PORT must be a number")?,
        })
    }

    /// Parse `"degen=http://degen:9101/metrics,imagine=..."`.
    fn parse_targets(raw: &str) -> Vec<BotTarget> {
        raw.split(',')
            .filter_map(|pair| {
                let (name, url) = pair.trim().split_once('=')?;
                Some(BotTarget {
                    name: name.trim().to_string(),
                    url: url.trim().to_string(),
                })
            })
            .collect()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this service's own .env (gitignored, per machine).
    botkit::Env::load_file(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    botkit::Telemetry::init("monitor");

    let config = Config::from_env()?;
    let db = db::Db::open(&config.db_path).await?;

    tracing::info!(
        "monitor started ({} bot(s), db: {}, port: {})",
        config.bots.len(),
        config.db_path,
        config.port
    );

    poller::Poller::spawn(config.bots.clone(), db.clone());
    api::serve(config.port, db).await
}
