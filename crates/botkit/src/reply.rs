//! The reply model: what a command wants the bot to do, and the single
//! place that executes it.

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

/// What a command wants the bot to do. Interpreted by [`dispatch`] —
/// commands never call `send_message`.
pub enum Reply {
    /// Send this block as a text message (capped at 4096).
    Text(Block),

    /// Deliver a photo with an optional caption (capped at 1024).
    Photo {
        bytes: Vec<u8>,
        caption: Option<String>,
    },

    /// Acknowledge with `placeholder`, run `job` in the background under
    /// supervision, then send its reply (or a uniform `⚠️` error).
    Background { placeholder: &'static str, job: Job },
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
    pub bot: Bot,
    pub msg: Message,
    /// The acknowledged placeholder message.
    pub placeholder: Message,
    pub chat_id: i64,
    pub user_id: Option<i64>,
}

/// The dispatch glue: job supervision and runtime metrics, injected as a
/// single dependency into handlers.
#[derive(Clone)]
pub struct Runtime {
    pub supervisor: Supervisor,
    pub metrics: Metrics,
}

/// Tracks in-flight background jobs so shutdown can drain them.
#[derive(Clone)]
pub struct Supervisor {
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
    pub async fn spawn(&self, job: Job, ctx: JobCtx) {
        self.metrics.job_started();
        let metrics = self.metrics.clone();
        let bot = ctx.bot.clone();
        let msg = ctx.msg.clone();
        let placeholder = ctx.placeholder.clone();
        let mut join_set = self.inner.lock().await;
        join_set.spawn(async move {
            // The job runs on its own task, so a panic surfaces as a
            // JoinError instead of killing the delivery.
            let handle = tokio::spawn((job.run)(ctx));
            let result = tokio::time::timeout(job.timeout, handle).await;
            let outcome = Self::job_outcome(result);
            let failed = outcome.is_err();
            Self::deliver(&bot, &msg, &placeholder, outcome).await;
            metrics.job_finished(failed);
        });
    }

    /// Wait for in-flight jobs, up to `grace`; abandon the rest.
    pub async fn drain(&self, grace: Duration) {
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
pub async fn dispatch<F>(
    bot: &Bot,
    msg: &Message,
    runtime: &Runtime,
    reply: F,
) -> ResponseResult<()>
where
    F: Future<Output = Result<Reply>>,
{
    runtime.metrics.note_update();
    match reply.await {
        Ok(Reply::Text(block)) => {
            bot.send_message(msg.chat.id, block.truncate(MAX_MESSAGE_LEN).build())
                .await?;
        }
        Ok(Reply::Photo { bytes, caption }) => {
            Reply::send_photo(bot, msg, bytes, caption).await?;
        }
        Ok(Reply::Background { placeholder, job }) => {
            let placeholder = bot.send_message(msg.chat.id, placeholder).await?;
            let ctx = JobCtx {
                bot: bot.clone(),
                msg: msg.clone(),
                placeholder,
                chat_id: msg.chat.id.0,
                user_id: msg.from.as_ref().map(|u| u.id.0 as i64),
            };
            runtime.supervisor.spawn(job, ctx).await;
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
}
