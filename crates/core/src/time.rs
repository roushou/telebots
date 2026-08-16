//! Wall-clock time helpers shared across bots and the monitor.

use std::time::{SystemTime, UNIX_EPOCH};

/// Wall-clock helpers.
pub struct Time;

impl Time {
    /// Seconds since the Unix epoch, clamped to zero on clock skew.
    pub fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}
