//! The admin monitor: polls every bot's `/metrics` endpoint, keeps status
//! snapshots in SQLite, and serves the dashboard + JSON API.

mod alerter;
mod api;
mod db;
mod health;
mod poller;
mod stats;

use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;

/// One bot to watch: its display name and `/metrics` URL.
#[derive(Debug, Clone)]
pub struct BotTarget {
    pub name: String,
    pub url: String,
}

/// Default database path used when `MONITOR_DB_PATH` is unset.
pub const DEFAULT_DB_PATH: &str = "monitor.db";

/// Default port used when `MONITOR_PORT` is unset.
pub const DEFAULT_PORT: u16 = 9110;

/// Default snapshot retention, in days.
pub const DEFAULT_RETENTION_DAYS: u64 = 30;

/// Fully validated runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub bots: Vec<BotTarget>,
    pub db_path: String,
    pub port: u16,
    pub retention_days: u64,
    pub alert_token: Option<String>,
    pub alert_chat_id: Option<String>,
}

/// The raw environment variables, deserialized by the `config` crate.
#[derive(Deserialize)]
struct RawEnv {
    /// Comma-separated `name=url` pairs (`MONITOR_BOTS`).
    #[serde(rename = "monitor_bots")]
    bots: String,
    /// SQLite path (`MONITOR_DB_PATH`).
    #[serde(rename = "monitor_db_path", default = "default_db_path")]
    db_path: String,
    /// HTTP port (`MONITOR_PORT`).
    #[serde(rename = "monitor_port", default = "default_port")]
    port: u16,
    /// Snapshot retention in days (`MONITOR_RETENTION_DAYS`).
    #[serde(rename = "monitor_retention_days", default = "default_retention_days")]
    retention_days: u64,
    /// Telegram bot token for alerts (`MONITOR_ALERT_TELEGRAM_TOKEN`);
    /// alerts are disabled when unset.
    #[serde(rename = "monitor_alert_telegram_token", default)]
    alert_token: Option<String>,
    /// Chat to send alerts to (`MONITOR_ALERT_CHAT_ID`).
    #[serde(rename = "monitor_alert_chat_id", default)]
    alert_chat_id: Option<String>,
}

fn default_db_path() -> String {
    DEFAULT_DB_PATH.to_string()
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

fn default_retention_days() -> u64 {
    DEFAULT_RETENTION_DAYS
}

impl Config {
    /// Load from the process environment (the caller loads `.env` first).
    pub fn from_env() -> Result<Self> {
        let raw: RawEnv = config::Config::builder()
            .add_source(
                config::Environment::default()
                    .ignore_empty(true)
                    .try_parsing(true),
            )
            .build()
            .context("failed to read the environment")?
            .try_deserialize()
            .context("invalid configuration")?;

        let bots = Self::parse_targets(&raw.bots);
        if bots.is_empty() {
            anyhow::bail!("MONITOR_BOTS has no valid name=url entries");
        }
        Ok(Self {
            bots,
            db_path: raw.db_path,
            port: raw.port,
            retention_days: raw.retention_days,
            alert_token: raw.alert_token,
            alert_chat_id: raw.alert_chat_id,
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
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init("monitor");

    let config = Config::from_env()?;
    let db = db::Db::open(&config.db_path).await?;

    tracing::info!(
        "monitor started ({} bot(s), db: {}, port: {})",
        config.bots.len(),
        config.db_path,
        config.port
    );

    let alerter = match (config.alert_token.as_ref(), config.alert_chat_id.as_ref()) {
        (Some(token), Some(chat_id)) => {
            tracing::info!("alerts enabled (telegram chat {chat_id})");
            Some(alerter::Alerter::new(token.clone(), chat_id.clone()))
        }
        _ => None,
    };

    let stats = stats::Stats::new();
    poller::Poller::spawn(config.bots.clone(), db.clone(), alerter, stats.clone());

    // Prune snapshots past the retention window, at startup and daily.
    let retention_days = config.retention_days;
    let prune_db = db.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(86_400));
        loop {
            tick.tick().await;
            if let Err(e) = prune_db.prune(retention_days).await {
                tracing::warn!("snapshot prune failed: {e:#}");
            }
        }
    });

    api::serve(config.port, db, stats, config.bots.len()).await
}
