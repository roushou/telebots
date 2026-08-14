//! Periodically fetch each bot's `/metrics` and store a snapshot.

use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use crate::{BotTarget, alerter::Alerter, db::Db};

/// How often each bot's status is fetched.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// The snapshot poller.
pub struct Poller;

impl Poller {
    /// Poll `bots` every [`POLL_INTERVAL`] in the background.
    pub fn spawn(bots: Vec<BotTarget>, db: Db, alerter: Option<Alerter>) {
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut tick = tokio::time::interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                for target in &bots {
                    if let Err(e) = Self::poll(&client, &db, &alerter, target).await {
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
        if let Some(alerter) = alerter {
            alerter
                .observe(&target.name, status.as_ref(), error.as_deref())
                .await;
        }
        Ok(())
    }
}
