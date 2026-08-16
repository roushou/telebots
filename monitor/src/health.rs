//! Bot health, derived from a snapshot's raw `/metrics` JSON. The bot is
//! authoritative: its payload reports `healthy`, and the monitor just reads
//! it, so alerting and the dashboard always agree with the bot.

use serde_json::Value;

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
/// but the bot reports itself unhealthy, `ok` otherwise.
pub fn health_of(status: Option<&Value>, error: Option<&str>) -> Health {
    if status.is_none() || error.is_some() {
        return Health::Down;
    }
    let status = status.expect("checked above");
    if status.get("healthy").and_then(Value::as_bool) == Some(true) {
        Health::Ok
    } else {
        Health::Degraded
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
    fn ok_when_reachable_and_healthy() {
        let status = json!({ "healthy": true });
        assert_eq!(health_of(Some(&status), None), Health::Ok);
    }

    #[test]
    fn degraded_when_reachable_but_unhealthy() {
        let status = json!({ "healthy": false });
        assert_eq!(health_of(Some(&status), None), Health::Degraded);

        // Old snapshots without the field default to degraded.
        let legacy = json!({});
        assert_eq!(health_of(Some(&legacy), None), Health::Degraded);
    }
}
