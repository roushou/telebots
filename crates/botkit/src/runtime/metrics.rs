//! Runtime metrics: status gauges and counters shared by the health
//! endpoint and the monitor.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI64, AtomicU64, AtomicUsize, Ordering},
    },
    time::Instant,
};

use serde::Serialize;

use crate::bot::STALE_AFTER_SECS;

/// Per-command counters.
#[derive(Serialize)]
pub struct CommandHealth {
    pub total: u64,
    pub errors: u64,
}

/// A point-in-time status snapshot, serialized for `/healthz` and
/// `/metrics`.
#[derive(Serialize)]
pub struct Health {
    pub service: &'static str,
    pub version: &'static str,
    pub uptime_secs: u64,
    pub telegram: &'static str,
    /// Whether the bot considers itself healthy: Telegram reachable and the
    /// heartbeat fresh. The monitor reads this directly.
    pub healthy: bool,
    pub last_heartbeat_ago_secs: Option<i64>,
    pub last_command_ago_secs: Option<i64>,
    pub commands_total: u64,
    pub dispatch_errors_total: u64,
    pub jobs_active: usize,
    pub jobs_failed_total: u64,
    pub panics_total: u64,
    /// Total prompt (input) tokens across LLM requests.
    pub llm_prompt_tokens_total: u64,
    /// Total completion (output) tokens across LLM requests.
    pub llm_completion_tokens_total: u64,
    /// Number of LLM requests made.
    pub llm_requests_total: u64,
    /// Cumulative LLM cost in micro-USD (millionths of a dollar).
    pub llm_cost_micro_usd_total: u64,
    pub commands: HashMap<&'static str, CommandHealth>,
}

/// Per-command counters, keyed by command name.
#[derive(Default)]
struct CommandStats {
    total: AtomicU64,
    errors: AtomicU64,
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
    llm_prompt_tokens: Arc<AtomicU64>,
    llm_completion_tokens: Arc<AtomicU64>,
    llm_requests: Arc<AtomicU64>,
    llm_cost_micro_usd: Arc<AtomicU64>,
    commands: Arc<Mutex<HashMap<&'static str, CommandStats>>>,
}

/// A cheap-to-clone handle for reporting LLM usage from background jobs.
///
/// Cost is caller-computed (in micro-USD) because pricing is model-specific
/// and lives outside the framework.
#[derive(Clone)]
pub struct UsageReporter {
    prompt_tokens: Arc<AtomicU64>,
    completion_tokens: Arc<AtomicU64>,
    requests: Arc<AtomicU64>,
    cost_micro_usd: Arc<AtomicU64>,
}

impl UsageReporter {
    /// Record one LLM request's token usage and cost.
    pub fn report(&self, prompt_tokens: u64, completion_tokens: u64, cost_micro_usd: u64) {
        self.prompt_tokens
            .fetch_add(prompt_tokens, Ordering::Relaxed);
        self.completion_tokens
            .fetch_add(completion_tokens, Ordering::Relaxed);
        self.requests.fetch_add(1, Ordering::Relaxed);
        self.cost_micro_usd
            .fetch_add(cost_micro_usd, Ordering::Relaxed);
    }
}

impl Metrics {
    pub fn new(service: &'static str, version: &'static str) -> Self {
        let now = telebots_core::Time::now_secs();
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
            llm_prompt_tokens: Arc::new(AtomicU64::new(0)),
            llm_completion_tokens: Arc::new(AtomicU64::new(0)),
            llm_requests: Arc::new(AtomicU64::new(0)),
            llm_cost_micro_usd: Arc::new(AtomicU64::new(0)),
            commands: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// A handle for reporting LLM usage, sharing this metrics' counters.
    pub fn usage_reporter(&self) -> UsageReporter {
        UsageReporter {
            prompt_tokens: self.llm_prompt_tokens.clone(),
            completion_tokens: self.llm_completion_tokens.clone(),
            requests: self.llm_requests.clone(),
            cost_micro_usd: self.llm_cost_micro_usd.clone(),
        }
    }

    /// A Telegram heartbeat succeeded.
    pub fn heartbeat_ok(&self) {
        self.telegram_ok.store(true, Ordering::Relaxed);
        self.last_heartbeat
            .store(telebots_core::Time::now_secs(), Ordering::Relaxed);
    }

    /// A Telegram heartbeat failed (transient failures self-heal; the
    /// staleness check in [`Metrics::alive`] decides liveness).
    pub fn heartbeat_failed(&self) {
        self.telegram_ok.store(false, Ordering::Relaxed);
    }

    /// A command was dispatched.
    pub fn note_command(&self) {
        self.last_command
            .store(telebots_core::Time::now_secs(), Ordering::Relaxed);
        self.commands_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Record one execution of the named command.
    pub fn note_command_named(&self, name: &'static str) {
        self.commands
            .lock()
            .unwrap()
            .entry(name)
            .or_default()
            .total
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record one failed execution of the named command.
    pub fn note_command_error(&self, name: &'static str) {
        self.commands
            .lock()
            .unwrap()
            .entry(name)
            .or_default()
            .errors
            .fetch_add(1, Ordering::Relaxed);
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
        let now = telebots_core::Time::now_secs();
        let ago = |t: i64| Some((now - t).max(0));
        let commands = self
            .commands
            .lock()
            .unwrap()
            .iter()
            .map(|(name, stats)| {
                (
                    *name,
                    CommandHealth {
                        total: stats.total.load(Ordering::Relaxed),
                        errors: stats.errors.load(Ordering::Relaxed),
                    },
                )
            })
            .collect();
        Health {
            service: self.service,
            version: self.version,
            uptime_secs: self.started.elapsed().as_secs(),
            telegram: if self.telegram_ok.load(Ordering::Relaxed) {
                "ok"
            } else {
                "unreachable"
            },
            healthy: self.telegram_ok.load(Ordering::Relaxed)
                && self.last_heartbeat.load(Ordering::Relaxed) + STALE_AFTER_SECS >= now,
            last_heartbeat_ago_secs: ago(self.last_heartbeat.load(Ordering::Relaxed)),
            last_command_ago_secs: ago(self.last_command.load(Ordering::Relaxed)),
            commands_total: self.commands_total.load(Ordering::Relaxed),
            dispatch_errors_total: self.dispatch_errors.load(Ordering::Relaxed),
            jobs_active: self.jobs_active.load(Ordering::Relaxed),
            jobs_failed_total: self.jobs_failed.load(Ordering::Relaxed),
            panics_total: self.panics.load(Ordering::Relaxed),
            llm_prompt_tokens_total: self.llm_prompt_tokens.load(Ordering::Relaxed),
            llm_completion_tokens_total: self.llm_completion_tokens.load(Ordering::Relaxed),
            llm_requests_total: self.llm_requests.load(Ordering::Relaxed),
            llm_cost_micro_usd_total: self.llm_cost_micro_usd.load(Ordering::Relaxed),
            commands,
        }
    }

    /// Liveness: a heartbeat was seen recently.
    pub fn alive(&self) -> bool {
        self.last_heartbeat.load(Ordering::Relaxed) + STALE_AFTER_SECS
            >= telebots_core::Time::now_secs()
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
        assert!(h.healthy);
        assert_eq!(h.jobs_active, 0);
        assert!(m.alive());
    }

    #[test]
    fn failed_heartbeat_flips_telegram_flag() {
        let m = Metrics::new("test", "0.1.0");
        m.heartbeat_failed();
        assert_eq!(m.health().telegram, "unreachable");
        assert!(!m.health().healthy);
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

    #[test]
    fn per_command_counters_track_executions_and_errors() {
        let m = Metrics::new("test", "0.1.0");
        m.note_command_named("price");
        m.note_command_named("price");
        m.note_command_named("info");
        m.note_command_error("price");
        let h = m.health();
        assert_eq!(h.commands["price"].total, 2);
        assert_eq!(h.commands["price"].errors, 1);
        assert_eq!(h.commands["info"].total, 1);
        assert_eq!(h.commands["info"].errors, 0);
    }

    #[test]
    fn usage_reporter_accumulates_tokens_and_cost() {
        let m = Metrics::new("test", "0.1.0");
        let usage = m.usage_reporter();
        usage.report(10, 20, 500);
        usage.report(5, 15, 250);
        let h = m.health();
        assert_eq!(h.llm_prompt_tokens_total, 15);
        assert_eq!(h.llm_completion_tokens_total, 35);
        assert_eq!(h.llm_requests_total, 2);
        assert_eq!(h.llm_cost_micro_usd_total, 750);
    }
}
