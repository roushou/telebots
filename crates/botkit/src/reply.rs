//! The reply model: what a command wants the bot to do, and the single
//! place that executes it.
//!
//! [`Reply::Background`] jobs and the command layer's results carry
//! [`anyhow::Result`] because command errors are authored with `anyhow` in
//! the binaries; botkit only transports and renders them (`⚠️ {e:#}`).
//!
//! Everything teloxide-typed (`Bot`, `Message`, `Runtime`, `Supervisor`,
//! `dispatch`) is crate-private — the public surface is the `Reply`/`Job`
//! outcomes commands produce.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use telebots_core::Block;
use teloxide::{
    prelude::*,
    types::{InputFile, ReplyParameters},
};
use tokio::{
    task::{JoinError, JoinSet},
    time::error::Elapsed,
};

use crate::metrics::Metrics;

/// Telegram's text message length limit.
const MAX_MESSAGE_LEN: usize = 4096;

/// Telegram's photo caption length limit.
const MAX_CAPTION_LEN: usize = 1024;

/// A boxed, sendable future.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// What a command wants the bot to do. Interpreted by botkit's single send
/// point — commands never call `send_message`.
#[non_exhaustive]
pub enum Reply {
    /// Send this block as a text message (capped at 4096).
    Text(Block),

    /// Deliver a photo with an optional caption (capped at 1024).
    Photo {
        bytes: Vec<u8>,
        caption: Option<String>,
    },

    /// Edit the acknowledgement placeholder in place (background jobs
    /// only). In the direct path there is nothing to edit, so it falls back
    /// to a normal text message.
    Edit(Block),

    /// Acknowledge with `placeholder`, run `job` in the background under
    /// supervision, then deliver its reply (or a uniform `⚠️` error).
    Background { placeholder: String, job: Job },
}

impl Reply {
    /// Send a photo with an optional caption, replying to `msg`.
    async fn send_photo(
        bot: &Bot,
        msg: &Message,
        bytes: Vec<u8>,
        caption: Option<String>,
    ) -> ResponseResult<Message> {
        let mut request = bot
            .send_photo(msg.chat.id, InputFile::memory(bytes))
            .reply_parameters(ReplyParameters::new(msg.id));
        if let Some(caption) = caption {
            request = request.caption(Self::cap_caption(caption));
        }
        request.await
    }

    /// Cap a caption at Telegram's limit.
    fn cap_caption(caption: String) -> String {
        if caption.chars().count() > MAX_CAPTION_LEN {
            let mut out: String = caption.chars().take(MAX_CAPTION_LEN - 1).collect();
            out.push('…');
            out
        } else {
            caption
        }
    }
}

/// A background job: what to run, and how long to let it run.
pub struct Job {
    pub timeout: Duration,
    run: Box<dyn FnOnce(JobCtx) -> BoxFuture<Result<Reply>> + Send>,
}

impl Job {
    /// A job that must finish within `timeout`; `run` produces the reply.
    pub fn new(
        timeout: Duration,
        run: impl FnOnce(JobCtx) -> BoxFuture<Result<Reply>> + Send + 'static,
    ) -> Self {
        Self {
            timeout,
            run: Box::new(run),
        }
    }
}

/// Everything a background job gets to finish the interaction.
pub struct JobCtx {
    pub chat_id: i64,
    pub user_id: Option<i64>,
}

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

    /// Run `job` in the background; deliver its reply (or a uniform error)
    /// and clean up the placeholder.
    pub(crate) async fn spawn(
        &self,
        job: Job,
        ctx: JobCtx,
        bot: Bot,
        msg: Message,
        placeholder: Message,
    ) {
        self.metrics.job_started();
        let metrics = self.metrics.clone();
        let mut join_set = self.inner.lock().await;
        join_set.spawn(async move {
            let outcome = Self::run_job(job, ctx).await;
            let failed = outcome.is_err();
            Self::deliver(&bot, &msg, &placeholder, outcome).await;
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
    async fn deliver(bot: &Bot, msg: &Message, placeholder: &Message, outcome: Result<Reply>) {
        match outcome {
            Ok(Reply::Edit(block)) => {
                // The edit replaces the placeholder, so there is nothing
                // left to delete.
                if let Err(e) = bot
                    .edit_message_text(
                        placeholder.chat.id,
                        placeholder.id,
                        block.truncate(MAX_MESSAGE_LEN).build(),
                    )
                    .await
                {
                    tracing::warn!("failed to edit placeholder: {e}");
                }
                return;
            }
            Ok(Reply::Text(block)) => {
                if let Err(e) = bot
                    .send_message(msg.chat.id, block.truncate(MAX_MESSAGE_LEN).build())
                    .await
                {
                    tracing::warn!("failed to send job reply: {e}");
                }
            }
            Ok(Reply::Photo { bytes, caption }) => {
                if let Err(e) = Reply::send_photo(bot, msg, bytes, caption).await {
                    tracing::warn!("failed to deliver photo: {e}");
                    let _ = bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await;
                }
            }
            Ok(Reply::Background { .. }) => {
                tracing::error!("background job returned a Background reply");
            }
            Err(e) => {
                tracing::warn!(chat_id = msg.chat.id.0, "background job failed: {e:#}");
                let _ = bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await;
            }
        }
        if let Err(e) = bot.delete_message(msg.chat.id, placeholder.id).await {
            tracing::warn!("failed to delete placeholder: {e}");
        }
    }
}

/// The single send point: interpret a command's [`Reply`], send it (with
/// Telegram's limits), or start a supervised background job. Errors render
/// as a uniform `⚠️` message.
pub(crate) async fn dispatch<F>(
    bot: &Bot,
    msg: &Message,
    supervisor: &Supervisor,
    reply: F,
) -> ResponseResult<()>
where
    F: Future<Output = Result<Reply>>,
{
    supervisor.metrics.note_command();
    match reply.await {
        Ok(Reply::Text(block)) => {
            bot.send_message(msg.chat.id, block.truncate(MAX_MESSAGE_LEN).build())
                .await?;
        }
        Ok(Reply::Photo { bytes, caption }) => {
            Reply::send_photo(bot, msg, bytes, caption).await?;
        }
        Ok(Reply::Edit(block)) => {
            // Nothing to edit in the direct path; fall back to a message.
            bot.send_message(msg.chat.id, block.truncate(MAX_MESSAGE_LEN).build())
                .await?;
        }
        Ok(Reply::Background { placeholder, job }) => {
            let placeholder = bot.send_message(msg.chat.id, placeholder).await?;
            let ctx = JobCtx {
                chat_id: msg.chat.id.0,
                user_id: msg.from.as_ref().map(|u| u.id.0 as i64),
            };
            supervisor
                .spawn(job, ctx, bot.clone(), msg.clone(), placeholder)
                .await;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_capped_at_telegram_limit() {
        let long = "x".repeat(MAX_CAPTION_LEN + 10);
        let capped = Reply::cap_caption(long);
        assert!(capped.chars().count() <= MAX_CAPTION_LEN);
        assert!(capped.ends_with('…'));

        let short = "ok".to_string();
        assert_eq!(Reply::cap_caption(short), "ok");
    }

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
        use std::sync::atomic::{AtomicBool, Ordering};

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
                Ok(Reply::Text(Block::new()))
            })
        });

        let outcome = Supervisor::run_job(
            job,
            JobCtx {
                chat_id: 1,
                user_id: None,
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
