//! Background-job supervision: run jobs under a deadline, deliver their
//! outcome, and drain in-flight jobs on shutdown.

use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use teloxide::types::{ChatId, MessageId};
use tokio::{
    task::{JoinError, JoinSet},
    time::error::Elapsed,
};

use crate::{
    dispatch::MAX_MESSAGE_LEN,
    messenger::Messenger,
    metrics::{Health, Metrics},
    reply::{Job, JobCtx, Reply},
};

/// Tracks in-flight background jobs so shutdown can drain them, and carries
/// the runtime metrics. Injected as a single dependency into handlers.
#[derive(Clone)]
pub(crate) struct Supervisor {
    inner: Arc<tokio::sync::Mutex<JoinSet<()>>>,
    metrics: Metrics,
}

impl Supervisor {
    pub(crate) fn new(metrics: Metrics) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(JoinSet::new())),
            metrics,
        }
    }

    /// A command was dispatched.
    pub(crate) fn note_command(&self) {
        self.metrics.note_command();
    }

    /// Record one execution of the named command.
    pub(crate) fn note_command_named(&self, name: &'static str) {
        self.metrics.note_command_named(name);
    }

    /// Record one failed execution of the named command.
    pub(crate) fn note_command_error(&self, name: &'static str) {
        self.metrics.note_command_error(name);
    }

    /// The current metrics snapshot.
    pub(crate) fn health(&self) -> Health {
        self.metrics.health()
    }

    /// A handle for reporting LLM usage from background jobs.
    pub(crate) fn usage_reporter(&self) -> crate::metrics::UsageReporter {
        self.metrics.usage_reporter()
    }

    /// Run `job` in the background; deliver its reply (or a uniform error)
    /// and clean up the placeholder.
    pub(crate) async fn spawn<M: Messenger>(
        &self,
        job: Job,
        ctx: JobCtx,
        messenger: M,
        chat: ChatId,
        reply_to: MessageId,
        placeholder: MessageId,
    ) {
        self.metrics.job_started();
        let metrics = self.metrics.clone();
        let mut join_set = self.inner.lock().await;
        join_set.spawn(async move {
            let outcome = Self::run_job(job, ctx).await;
            let failed = outcome.is_err();
            Self::deliver(&messenger, chat, reply_to, placeholder, outcome).await;
            metrics.job_finished(failed);
        });
    }

    /// Run a job under its deadline, aborting it if it overruns. The job
    /// runs on its own task so a panic surfaces as a `JoinError` instead of
    /// killing the delivery.
    async fn run_job(job: Job, ctx: JobCtx) -> Result<Reply> {
        let mut handle = tokio::spawn((job.run)(ctx));
        let result = tokio::select! {
            result = &mut handle => Ok(result),
            _ = tokio::time::sleep(job.timeout) => Err(()),
        };
        match result {
            Ok(result) => Self::job_outcome(Ok(result)),
            Err(()) => {
                handle.abort();
                let _ = (&mut handle).await;
                Err(anyhow!("background job timed out"))
            }
        }
    }

    /// Wait for in-flight jobs, up to `grace`; abandon the rest.
    pub(crate) async fn drain(&self, grace: Duration) {
        let mut join_set = self.inner.lock().await;
        let deadline = tokio::time::Instant::now() + grace;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, join_set.join_next()).await {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => break,
            }
        }
    }

    /// Normalize the raw job result (job error / panic / timeout).
    fn job_outcome(
        result: Result<Result<Result<Reply, anyhow::Error>, JoinError>, Elapsed>,
    ) -> Result<Reply> {
        match result {
            Ok(Ok(Ok(reply))) => Ok(reply),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err(anyhow!("background job panicked")),
            Err(_) => Err(anyhow!("background job timed out")),
        }
    }

    /// Deliver the job's outcome; the placeholder is always cleaned up.
    async fn deliver<M: Messenger>(
        messenger: &M,
        chat: ChatId,
        reply_to: MessageId,
        placeholder: MessageId,
        outcome: Result<Reply>,
    ) {
        match outcome {
            Ok(Reply::Edit(block)) => {
                // The edit replaces the placeholder, so there is nothing
                // left to delete.
                if let Err(e) = messenger
                    .edit_text(chat, placeholder, block.truncate(MAX_MESSAGE_LEN).build())
                    .await
                {
                    tracing::warn!("failed to edit placeholder: {e}");
                }
                return;
            }
            Ok(Reply::Text { block, markup }) => {
                if let Err(e) = messenger
                    .send_text(chat, block.truncate(MAX_MESSAGE_LEN).build(), markup)
                    .await
                {
                    tracing::warn!("failed to send job reply: {e}");
                }
            }
            Ok(Reply::Photo {
                bytes,
                caption,
                markup,
            }) => {
                if let Err(e) = messenger
                    .send_photo(chat, reply_to, bytes, caption, markup)
                    .await
                {
                    tracing::warn!("failed to deliver photo: {e}");
                    let _ = messenger.send_text(chat, format!("⚠️ {e:#}"), None).await;
                }
            }
            Ok(Reply::Background { .. }) => {
                tracing::error!("background job returned a Background reply");
            }
            Err(e) => {
                tracing::warn!(chat_id = chat.0, "background job failed: {e:#}");
                let _ = messenger.send_text(chat, format!("⚠️ {e:#}"), None).await;
            }
        }
        if let Err(e) = messenger.delete(chat, placeholder).await {
            tracing::warn!("failed to delete placeholder: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use telebots_core::Block;

    use super::*;

    #[tokio::test]
    async fn job_outcome_maps_timeout_and_panic() {
        let err = anyhow::anyhow!("boom");
        assert_eq!(
            Supervisor::job_outcome(Ok(Ok(Err(err))))
                .err()
                .unwrap()
                .to_string(),
            "boom"
        );

        let elapsed = tokio::time::timeout(
            std::time::Duration::from_millis(1),
            std::future::pending::<()>(),
        )
        .await
        .unwrap_err();
        let msg = Supervisor::job_outcome(Err(elapsed))
            .err()
            .unwrap()
            .to_string();
        assert!(msg.contains("timed out"));

        let handle = tokio::spawn(async { panic!("boom") });
        let join_err = handle.await.unwrap_err();
        let msg = Supervisor::job_outcome(Ok(Err(join_err)))
            .err()
            .unwrap()
            .to_string();
        assert!(msg.contains("panicked"));
    }

    #[tokio::test]
    async fn timed_out_job_is_aborted() {
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = dropped.clone();
        let job = Job::new(Duration::from_millis(1), move |_ctx| {
            Box::pin(async move {
                struct Guard(Arc<AtomicBool>);
                impl Drop for Guard {
                    fn drop(&mut self) {
                        self.0.store(true, Ordering::SeqCst);
                    }
                }
                let _guard = Guard(flag);
                std::future::pending::<()>().await;
                Ok(Reply::text(Block::new()))
            })
        });

        let metrics = Metrics::new("test", "0.1.0");
        let outcome = Supervisor::run_job(
            job,
            JobCtx {
                chat_id: 1,
                user_id: None,
                usage: metrics.usage_reporter(),
            },
        )
        .await;
        assert!(outcome.is_err());
        assert!(
            dropped.load(Ordering::SeqCst),
            "a timed-out job's future should be dropped"
        );
    }
}
