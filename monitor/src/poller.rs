//! Periodically fetch each bot's `/metrics` and store a snapshot.

use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use crate::{BotTarget, db::Db};

/// How often each bot's status is fetched.
const POLL_INTERVAL: Duration = Duration::from_secs(30);

/// The snapshot poller.
pub struct Poller;

impl Poller {
    /// Poll `bots` every [`POLL_INTERVAL`] in the background.
    pub fn spawn(bots: Vec<BotTarget>, db: Db) {
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let mut tick = tokio::time::interval(POLL_INTERVAL);
            loop {
                tick.tick().await;
                for target in &bots {
                    if let Err(e) = Self::poll(&client, &db, target).await {
                        tracing::warn!("polling {} failed: {e:#}", target.name);
                    }
                }
            }
        });
    }

    /// Fetch one bot's status and record it; unreachable bots are recorded
    /// with the error.
    async fn poll(client: &reqwest::Client, db: &Db, target: &BotTarget) -> Result<()> {
        match client.get(&target.url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                Ok(status) => {
                    db.insert_snapshot(&target.name, Some(&status), None)
                        .await?
                }
                Err(e) => {
                    db.insert_snapshot(&target.name, None, Some(&format!("bad response: {e}")))
                        .await?;
                }
            },
            Ok(resp) => {
                db.insert_snapshot(&target.name, None, Some(&format!("HTTP {}", resp.status())))
                    .await?;
            }
            Err(e) => {
                db.insert_snapshot(&target.name, None, Some(&format!("{e}")))
                    .await?;
            }
        }
        Ok(())
    }
}
