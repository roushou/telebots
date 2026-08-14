//! The admin monitor: polls every bot's `/metrics` endpoint, keeps status
//! snapshots in SQLite, and serves the dashboard + JSON API.

mod alerter;
mod api;
mod config;
mod db;
mod health;
mod poller;
mod stats;

use std::time::Duration;

use crate::config::Config;

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
