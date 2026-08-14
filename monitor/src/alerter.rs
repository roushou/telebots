//! Telegram alerts on bot health transitions, with a cooldown and a
//! "still down" reminder.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::health::{Health, health_of};

/// At most one alert per bot within this window.
const COOLDOWN: Duration = Duration::from_secs(15 * 60);

/// Re-alert if a bot stays down/degraded this long.
const REMINDER: Duration = Duration::from_secs(6 * 3600);

struct BotState {
    health: Health,
    since: Instant,
    last_alert_at: Option<Instant>,
}

/// Sends Telegram messages when a bot's health changes.
pub struct Alerter {
    client: reqwest::Client,
    api_url: String,
    chat_id: String,
    state: Mutex<HashMap<String, BotState>>,
}

impl Alerter {
    /// An alerter that posts `sendMessage` to `chat_id` with `token`.
    pub fn new(token: String, chat_id: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: format!("https://api.telegram.org/bot{token}/sendMessage"),
            chat_id,
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Observe a fresh snapshot and alert on health transitions.
    pub async fn observe(&self, name: &str, status: Option<&Value>, error: Option<&str>) {
        let health = health_of(status, error);
        if let Some(text) = self.next_alert(name, health, error).await {
            self.send(&text).await;
        }
    }

    /// Compute the next alert message (if any), updating in-memory state.
    async fn next_alert(&self, name: &str, health: Health, error: Option<&str>) -> Option<String> {
        let mut states = self.state.lock().await;
        let state = states.entry(name.to_string()).or_insert_with(|| BotState {
            health,
            since: Instant::now(),
            last_alert_at: None,
        });

        if state.health == health {
            // A persistent outage: remind every so often.
            if health != Health::Ok && state.since.elapsed() >= REMINDER {
                state.since = Instant::now();
                if Self::can_alert(state.last_alert_at) {
                    state.last_alert_at = Some(Instant::now());
                    return Some(format!(
                        "{} is still {} — {}",
                        name,
                        health.label(),
                        reason(error)
                    ));
                }
            }
            return None;
        }

        let previous = state.health;
        let previous_duration = state.since.elapsed();
        state.health = health;
        state.since = Instant::now();

        if Self::can_alert(state.last_alert_at) {
            state.last_alert_at = Some(Instant::now());
            return Some(transition(name, previous, health, previous_duration, error));
        }
        None
    }

    fn can_alert(last_alert_at: Option<Instant>) -> bool {
        last_alert_at.is_none_or(|t| t.elapsed() >= COOLDOWN)
    }

    async fn send(&self, text: &str) {
        match self
            .client
            .post(&self.api_url)
            .json(&json!({ "chat_id": self.chat_id, "text": text }))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => tracing::warn!("alert send failed: HTTP {}", resp.status()),
            Err(e) => tracing::warn!("alert send failed: {e}"),
        }
    }
}

fn reason(error: Option<&str>) -> &str {
    error.unwrap_or("no response")
}

fn transition(
    name: &str,
    prev: Health,
    next: Health,
    duration: Duration,
    error: Option<&str>,
) -> String {
    match (prev, next) {
        (_, Health::Ok) => format!(
            "🟢 {name} recovered (was {} {})",
            prev.label(),
            humanize(duration)
        ),
        (Health::Ok, Health::Degraded) => format!("🟡 {name} can't reach Telegram"),
        (Health::Ok, Health::Down) | (Health::Degraded, Health::Down) => {
            format!("🔴 {name} is down — {}", reason(error))
        }
        (Health::Down, Health::Degraded) => {
            format!("🟡 {name} is reachable but can't reach Telegram")
        }
        _ => unreachable!("same-state transitions are handled before calling transition"),
    }
}

fn humanize(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_messages() {
        let d = Duration::from_secs(12 * 60);
        assert_eq!(
            transition("degen", Health::Ok, Health::Down, d, Some("HTTP 500")),
            "🔴 degen is down — HTTP 500"
        );
        assert_eq!(
            transition("degen", Health::Down, Health::Ok, d, None),
            "🟢 degen recovered (was down 12m)"
        );
        assert_eq!(
            transition("degen", Health::Ok, Health::Degraded, d, None),
            "🟡 degen can't reach Telegram"
        );
    }

    #[test]
    fn humanize_durations() {
        assert_eq!(humanize(Duration::from_secs(45)), "45s");
        assert_eq!(humanize(Duration::from_secs(12 * 60)), "12m");
        assert_eq!(humanize(Duration::from_secs(3 * 3600 + 5 * 60)), "3h 5m");
    }
}
