//! The single send point: interpret a [`Reply`] and deliver it (with
//! Telegram's limits) or start a supervised background job. Errors render
//! as a uniform `⚠️` message. Everything here is crate-private: bots produce
//! `Reply`s, they never touch this.

use anyhow::Result;
use teloxide::{
    requests::ResponseResult,
    types::{ChatId, MessageId},
};

use crate::{
    messenger::Messenger,
    reply::{JobCtx, Reply},
    supervisor::Supervisor,
};

/// Telegram's text message length limit.
pub(crate) const MAX_MESSAGE_LEN: usize = 4096;

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
    supervisor.note_command();
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
    use std::{sync::Arc, time::Duration};

    use telebots_core::Block;

    use super::*;
    use crate::{metrics::Metrics, reply::Job};

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
}
