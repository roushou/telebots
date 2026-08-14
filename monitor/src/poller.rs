//! Periodically fetch each bot's `/metrics` and store a snapshot.

use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use serde_json::Value;

use crate::{BotTarget, alerter::Alerter, db::Db, stats::Stats};

/// How often each bot's status is fetched.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// Per-request timeout, so one hung bot can't stall the poll loop.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The snapshot poller.
pub struct Poller;

impl Poller {
    /// Poll `bots` every [`POLL_INTERVAL`] in the background.
    pub fn spawn(bots: Vec<BotTarget>, db: Db, alerter: Option<Alerter>, stats: Stats) {
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .expect("static client options");
            let mut tick = tokio::time::interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                stats.note_poll_cycle().await;
                // Poll concurrently so one slow bot doesn't delay the rest.
                let futures = bots
                    .iter()
                    .map(|target| Self::poll(&client, &db, &alerter, &stats, target));
                for (target, result) in bots.iter().zip(join_all(futures).await) {
                    if let Err(e) = result {
                        stats.note_poll_error();
                        tracing::warn!("polling {} failed: {e:#}", target.name);
                    }
                }
            }
        });
    }

    /// Fetch one bot's status, record it, and alert on health changes.
    /// Unreachable bots are recorded with the error.
    async fn poll(
        client: &reqwest::Client,
        db: &Db,
        alerter: &Option<Alerter>,
        stats: &Stats,
        target: &BotTarget,
    ) -> Result<()> {
        let (status, error) = match client.get(&target.url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                Ok(status) => (Some(status), None),
                Err(e) => (None, Some(format!("bad response: {e}"))),
            },
            Ok(resp) => (None, Some(format!("HTTP {}", resp.status()))),
            Err(e) => (None, Some(format!("{e}"))),
        };

        db.insert_snapshot(&target.name, status.as_ref(), error.as_deref())
            .await?;
        stats.note_snapshot();
        if let Some(alerter) = alerter {
            alerter
                .observe(&target.name, status.as_ref(), error.as_deref())
                .await;
        }
        Ok(())
    }
}
