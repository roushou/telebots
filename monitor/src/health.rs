//! Bot health, derived from a snapshot's raw `/metrics` JSON. Mirrors the
//! dashboard's classification so alerting and display agree.

use serde_json::Value;

/// How stale a heartbeat may be before the bot is considered degraded
/// (matches botkit's liveness threshold).
const HEARTBEAT_STALE_SECS: i64 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Ok,
    Degraded,
    Down,
}

impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Health::Ok => "ok",
            Health::Degraded => "degraded",
            Health::Down => "down",
        }
    }
}

/// Classify a snapshot: `down` when unreachable, `degraded` when reachable
/// but its Telegram link is stale/unreachable, `ok` otherwise.
pub fn health_of(status: Option<&Value>, error: Option<&str>) -> Health {
    if status.is_none() || error.is_some() {
        return Health::Down;
    }
    let status = status.expect("checked above");
    let telegram_ok = status.get("telegram").and_then(Value::as_str) == Some("ok");
    let stale = status
        .get("last_heartbeat_ago_secs")
        .and_then(Value::as_i64)
        .is_some_and(|ago| ago > HEARTBEAT_STALE_SECS);
    if !telegram_ok || stale {
        Health::Degraded
    } else {
        Health::Ok
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn down_when_unreachable() {
        assert_eq!(health_of(None, Some("HTTP 500")), Health::Down);
        assert_eq!(health_of(None, None), Health::Down);
    }

    #[test]
    fn ok_when_reachable_and_heartbeat_fresh() {
        let status = json!({ "telegram": "ok", "last_heartbeat_ago_secs": 10 });
        assert_eq!(health_of(Some(&status), None), Health::Ok);
    }

    #[test]
    fn degraded_when_telegram_unreachable_or_stale() {
        let unreachable = json!({ "telegram": "unreachable", "last_heartbeat_ago_secs": 10 });
        assert_eq!(health_of(Some(&unreachable), None), Health::Degraded);

        let stale = json!({ "telegram": "ok", "last_heartbeat_ago_secs": 200 });
        assert_eq!(health_of(Some(&stale), None), Health::Degraded);
    }
}
