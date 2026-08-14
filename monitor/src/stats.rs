//! The monitor's own runtime stats — the monitor is monitored too.

use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use tokio::sync::Mutex;

/// Shared, cheap-to-clone poller stats.
#[derive(Clone, Default)]
pub struct Stats {
    inner: Arc<Inner>,
}

#[derive(Default)]
struct Inner {
    last_poll: Mutex<Option<Instant>>,
    poll_errors: AtomicU64,
    snapshots: AtomicU64,
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the start of a poll cycle.
    pub async fn note_poll_cycle(&self) {
        *self.inner.last_poll.lock().await = Some(Instant::now());
    }

    /// Record a poll failure (fetch or recording).
    pub fn note_poll_error(&self) {
        self.inner.poll_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a snapshot being stored.
    pub fn note_snapshot(&self) {
        self.inner.snapshots.fetch_add(1, Ordering::Relaxed);
    }

    /// Seconds since the last poll cycle started.
    pub async fn last_poll_ago_secs(&self) -> Option<u64> {
        self.inner
            .last_poll
            .lock()
            .await
            .map(|t| t.elapsed().as_secs())
    }

    /// Cumulative poll errors.
    pub fn poll_errors(&self) -> u64 {
        self.inner.poll_errors.load(Ordering::Relaxed)
    }

    /// Cumulative snapshots stored.
    pub fn snapshots(&self) -> u64 {
        self.inner.snapshots.load(Ordering::Relaxed)
    }
}
