//! Runtime metrics: status gauges and counters shared by the health
//! endpoint and the monitor.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;

/// How old a successful heartbeat may be before the bot is considered
/// dead (3× the 60s heartbeat interval).
const STALE_AFTER_SECS: i64 = 180;

/// A point-in-time status snapshot, serialized for `/healthz` and
/// `/metrics`.
#[derive(Serialize)]
pub struct Health {
    pub service: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
    pub telegram: &'static str,
    pub last_heartbeat_ago_secs: Option<i64>,
    pub last_command_ago_secs: Option<i64>,
    pub commands_total: u64,
    pub dispatch_errors_total: u64,
    pub jobs_active: usize,
    pub jobs_failed_total: u64,
    pub panics_total: u64,
}

/// Shared runtime metrics for one bot process.
#[derive(Clone)]
pub struct Metrics {
    service: &'static str,
    version: &'static str,
    started: Instant,
    telegram_ok: Arc<AtomicBool>,
    last_heartbeat: Arc<AtomicI64>,
    last_command: Arc<AtomicI64>,
    commands_total: Arc<AtomicU64>,
    dispatch_errors: Arc<AtomicU64>,
    jobs_active: Arc<AtomicUsize>,
    jobs_failed: Arc<AtomicU64>,
    panics: Arc<AtomicU64>,
}

impl Metrics {
    pub fn new(service: &'static str, version: &'static str) -> Self {
        let now = Self::now_unix();
        Self {
            service,
            version,
            started: Instant::now(),
            telegram_ok: Arc::new(AtomicBool::new(true)),
            last_heartbeat: Arc::new(AtomicI64::new(now)),
            last_command: Arc::new(AtomicI64::new(now)),
            commands_total: Arc::new(AtomicU64::new(0)),
            dispatch_errors: Arc::new(AtomicU64::new(0)),
            jobs_active: Arc::new(AtomicUsize::new(0)),
            jobs_failed: Arc::new(AtomicU64::new(0)),
            panics: Arc::new(AtomicU64::new(0)),
        }
    }

    /// A Telegram heartbeat succeeded.
    pub fn heartbeat_ok(&self) {
        self.telegram_ok.store(true, Ordering::Relaxed);
        self.last_heartbeat
            .store(Self::now_unix(), Ordering::Relaxed);
    }

    /// A Telegram heartbeat failed (transient failures self-heal; the
    /// staleness check in [`Metrics::alive`] decides liveness).
    pub fn heartbeat_failed(&self) {
        self.telegram_ok.store(false, Ordering::Relaxed);
    }

    /// A command was dispatched.
    pub fn note_command(&self) {
        self.last_command.store(Self::now_unix(), Ordering::Relaxed);
        self.commands_total.fetch_add(1, Ordering::Relaxed);
    }

    /// A command failed to be delivered (the dispatcher's error handler
    /// observed a request error).
    pub fn note_dispatch_error(&self) {
        self.dispatch_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// A background job started.
    pub fn job_started(&self) {
        self.jobs_active.fetch_add(1, Ordering::Relaxed);
    }

    /// A background job finished; `failed` marks error/timeout/panic.
    pub fn job_finished(&self, failed: bool) {
        self.jobs_active.fetch_sub(1, Ordering::Relaxed);
        if failed {
            self.jobs_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// A process panic was observed.
    pub fn note_panic(&self) {
        self.panics.fetch_add(1, Ordering::Relaxed);
    }

    /// The current status snapshot.
    pub fn health(&self) -> Health {
        let now = Self::now_unix();
        let ago = |t: i64| Some((now - t).max(0));
        Health {
            service: self.service,
            version: self.version,
            uptime_secs: self.started.elapsed().as_secs(),
            telegram: if self.telegram_ok.load(Ordering::Relaxed) {
                "ok"
            } else {
                "unreachable"
            },
            last_heartbeat_ago_secs: ago(self.last_heartbeat.load(Ordering::Relaxed)),
            last_command_ago_secs: ago(self.last_command.load(Ordering::Relaxed)),
            commands_total: self.commands_total.load(Ordering::Relaxed),
            dispatch_errors_total: self.dispatch_errors.load(Ordering::Relaxed),
            jobs_active: self.jobs_active.load(Ordering::Relaxed),
            jobs_failed_total: self.jobs_failed.load(Ordering::Relaxed),
            panics_total: self.panics.load(Ordering::Relaxed),
        }
    }

    /// Liveness: a heartbeat was seen recently.
    pub fn alive(&self) -> bool {
        self.last_heartbeat.load(Ordering::Relaxed) + STALE_AFTER_SECS >= Self::now_unix()
    }

    fn now_unix() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_reflects_startup_state() {
        let m = Metrics::new("test", "0.1.0");
        let h = m.health();
        assert_eq!(h.service, "test");
        assert_eq!(h.version, "0.1.0");
        assert_eq!(h.telegram, "ok");
        assert_eq!(h.jobs_active, 0);
        assert!(m.alive());
    }

    #[test]
    fn failed_heartbeat_flips_telegram_flag() {
        let m = Metrics::new("test", "0.1.0");
        m.heartbeat_failed();
        assert_eq!(m.health().telegram, "unreachable");
        // Still alive until the last heartbeat goes stale.
        assert!(m.alive());
    }

    #[test]
    fn job_counters_track_activity() {
        let m = Metrics::new("test", "0.1.0");
        m.note_command();
        m.note_command();
        m.note_dispatch_error();
        m.job_started();
        m.job_started();
        m.job_finished(true);
        let h = m.health();
        assert_eq!(h.commands_total, 2);
        assert_eq!(h.dispatch_errors_total, 1);
        assert_eq!(h.jobs_active, 1);
        assert_eq!(h.jobs_failed_total, 1);
        m.note_panic();
        assert_eq!(m.health().panics_total, 1);
    }
}
