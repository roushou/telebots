//! The single send point and background-job supervision.
//!
//! [`dispatch`] interprets a [`crate::Reply`] and either delivers it through
//! a [`Messenger`] or runs a [`crate::reply::Job`] under [`Supervisor`],
//! which delivers the outcome and drains on shutdown. Everything here is
//! crate-private: bots produce `Reply`s, they never touch this.

use std::{sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use teloxide::{
    requests::ResponseResult,
    types::{ChatId, MessageId},
};
use tokio::{
    task::{JoinError, JoinSet},
    time::error::Elapsed,
};

use crate::{
    messenger::Messenger,
    metrics::{Health, Metrics},
    reply::{Job, JobCtx, Reply},
};

/// Telegram's text message length limit.
pub(crate) const MAX_MESSAGE_LEN: usize = 4096;

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

/// The single send point: interpret a command's [`Reply`], deliver it (with
/// Telegram's limits), or start a supervised background job. Errors render
/// as a uniform `⚠️` message.
pub(crate) async fn dispatch<M, F>(
    messenger: &M,
    chat: ChatId,
    reply_to: MessageId,
    user_id: Option<i64>,
    supervisor: &Supervisor,
    reply: F,
) -> ResponseResult<()>
where
    M: Messenger,
    F: Future<Output = Result<Reply>>,
{
    supervisor.metrics.note_command();
    match reply.await {
        Ok(Reply::Text { block, markup }) => {
            messenger
                .send_text(chat, block.truncate(MAX_MESSAGE_LEN).build(), markup)
                .await?;
        }
        Ok(Reply::Photo {
            bytes,
            caption,
            markup,
        }) => {
            messenger
                .send_photo(chat, reply_to, bytes, caption, markup)
                .await?;
        }
        Ok(Reply::Edit(block)) => {
            // Nothing to edit in the direct path; fall back to a message.
            messenger
                .send_text(chat, block.truncate(MAX_MESSAGE_LEN).build(), None)
                .await?;
        }
        Ok(Reply::Background { placeholder, job }) => {
            let placeholder_id = messenger.send_text(chat, placeholder, None).await?;
            let ctx = JobCtx {
                chat_id: chat.0,
                user_id,
                usage: supervisor.usage_reporter(),
            };
            supervisor
                .spawn(job, ctx, messenger.clone(), chat, reply_to, placeholder_id)
                .await;
        }
        Err(e) => {
            messenger.send_text(chat, format!("⚠️ {e:#}"), None).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, Ordering};

    use telebots_core::Block;

    use super::*;

    fn block(text: &str) -> Block {
        let mut b = Block::new();
        b.line(text);
        b
    }

    /// Records every delivery call; `ChatId(1)` is the only chat used.
    #[derive(Clone, Default)]
    struct Mock {
        calls: Arc<tokio::sync::Mutex<Vec<Call>>>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Call {
        SendText(String),
        SendPhoto(Option<String>),
        EditText(String),
        Delete,
        Answer,
    }

    #[crate::async_trait]
    impl Messenger for Mock {
        async fn send_text(
            &self,
            _chat: ChatId,
            text: String,
            _markup: Option<crate::markup::Markup>,
        ) -> ResponseResult<MessageId> {
            self.calls.lock().await.push(Call::SendText(text));
            Ok(MessageId(1))
        }

        async fn send_photo(
            &self,
            _chat: ChatId,
            _reply_to: MessageId,
            _bytes: Vec<u8>,
            caption: Option<String>,
            _markup: Option<crate::markup::Markup>,
        ) -> ResponseResult<MessageId> {
            self.calls.lock().await.push(Call::SendPhoto(caption));
            Ok(MessageId(2))
        }

        async fn edit_text(
            &self,
            _chat: ChatId,
            _msg: MessageId,
            text: String,
        ) -> ResponseResult<()> {
            self.calls.lock().await.push(Call::EditText(text));
            Ok(())
        }

        async fn delete(&self, _chat: ChatId, _msg: MessageId) -> ResponseResult<()> {
            self.calls.lock().await.push(Call::Delete);
            Ok(())
        }

        async fn answer_callback(
            &self,
            _query_id: teloxide::types::CallbackQueryId,
        ) -> ResponseResult<()> {
            self.calls.lock().await.push(Call::Answer);
            Ok(())
        }
    }

    async fn calls(mock: &Mock) -> Vec<Call> {
        mock.calls.lock().await.clone()
    }

    #[tokio::test]
    async fn dispatch_sends_text() {
        let messenger = Mock::default();
        let supervisor = Supervisor::new(Metrics::new("test", "0.1.0"));
        let reply = async { Ok(Reply::text(block("hello"))) };

        dispatch(
            &messenger,
            ChatId(1),
            MessageId(5),
            None,
            &supervisor,
            reply,
        )
        .await
        .unwrap();

        assert_eq!(
            calls(&messenger).await,
            vec![Call::SendText("hello".into())]
        );
    }

    #[tokio::test]
    async fn dispatch_renders_errors() {
        let messenger = Mock::default();
        let supervisor = Supervisor::new(Metrics::new("test", "0.1.0"));
        let reply = async { Err::<Reply, _>(anyhow::anyhow!("boom")) };

        dispatch(
            &messenger,
            ChatId(1),
            MessageId(5),
            None,
            &supervisor,
            reply,
        )
        .await
        .unwrap();

        let recorded = calls(&messenger).await;
        assert_eq!(recorded.len(), 1);
        assert!(matches!(&recorded[0], Call::SendText(t) if t == "⚠️ boom"));
    }

    #[tokio::test]
    async fn background_job_delivers_and_cleans_up() {
        let messenger = Mock::default();
        let supervisor = Supervisor::new(Metrics::new("test", "0.1.0"));
        let reply = async {
            Ok(Reply::Background {
                placeholder: "working…".into(),
                job: Job::new(Duration::from_secs(1), |_ctx| {
                    Box::pin(async { Ok(Reply::text(block("done"))) })
                }),
            })
        };

        dispatch(
            &messenger,
            ChatId(1),
            MessageId(5),
            Some(42),
            &supervisor,
            reply,
        )
        .await
        .unwrap();
        supervisor.drain(Duration::from_secs(1)).await;

        assert_eq!(
            calls(&messenger).await,
            vec![
                Call::SendText("working…".into()),
                Call::SendText("done".into()),
                Call::Delete,
            ]
        );
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
