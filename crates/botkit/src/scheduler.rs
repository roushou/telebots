//! Proactive delivery: a periodic loop that asks the bot what work is due
//! and sends it. This is how bots do things on a clock (reminders, feed
//! polls, price alerts) instead of only reacting to updates.
//!
//! The bot implements [`ScheduleSource`] (what is due now, and how to mark
//! it delivered) and hands it to [`crate::BotBuilder::scheduler`]; botkit
//! owns the tick, the transport, and the retry-until-delivered semantics.

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use telebots_core::Block;
use teloxide::{Bot as Api, types::ChatId};

use crate::runtime::{MAX_MESSAGE_LEN, Messenger};

/// One message the scheduler must deliver.
#[derive(Debug, Clone)]
pub struct ScheduledMessage {
    /// Bot-defined id, passed back to [`ScheduleSource::delivered`].
    pub id: i64,
    /// Telegram chat to send the message to.
    pub chat_id: i64,
    /// The message body.
    pub block: Block,
}

/// The bot's view of scheduled work. The scheduler owns the clock and the
/// transport; the source owns what "due" means and how to retire an item.
#[crate::async_trait]
pub trait ScheduleSource: Send + Sync + 'static {
    /// Items that are due now. Items that fail to send are returned again on
    /// a later tick, so they are retried until [`ScheduleSource::delivered`]
    /// is called for them.
    async fn due(&self) -> Result<Vec<ScheduledMessage>>;

    /// Mark one item delivered. The scheduler calls this only after the
    /// message reached Telegram, so a crash between send and mark can
    /// redeliver (at-least-once).
    async fn delivered(&self, id: i64) -> Result<()>;
}

/// The periodic delivery loop. Crate-private; spawned by [`crate::Bot::run`].
pub(crate) struct Scheduler {
    interval: Duration,
    source: Arc<dyn ScheduleSource>,
}

impl Scheduler {
    pub(crate) fn new(interval: Duration, source: Arc<dyn ScheduleSource>) -> Self {
        Self { interval, source }
    }

    /// Run the loop on a background task. The first tick fires immediately,
    /// so items that came due while the bot was offline are delivered on
    /// startup.
    pub(crate) fn spawn(self, api: Api) {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(self.interval);
            loop {
                tick.tick().await;
                self.run_once(&api).await;
            }
        });
    }

    /// Fetch due work, send each item, and mark the sent ones delivered.
    async fn run_once<M: Messenger>(&self, messenger: &M) {
        let due = match self.source.due().await {
            Ok(due) => due,
            Err(e) => {
                tracing::warn!("scheduler: failed to fetch due work: {e:#}");
                return;
            }
        };
        for item in due {
            let text = item.block.truncate(MAX_MESSAGE_LEN).build();
            match messenger.send_text(ChatId(item.chat_id), text, None).await {
                Ok(_) => {
                    if let Err(e) = self.source.delivered(item.id).await {
                        tracing::warn!(
                            id = item.id,
                            "scheduler: failed to mark item delivered: {e:#}"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(id = item.id, "scheduler: failed to send: {e}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use teloxide::{
        requests::ResponseResult,
        types::{CallbackQueryId, MessageId},
    };

    use super::*;
    use crate::markup::Markup;

    /// A schedule source that yields one fixed item and records deliveries.
    #[derive(Default)]
    struct Source {
        delivered: Mutex<Vec<i64>>,
    }

    #[crate::async_trait]
    impl ScheduleSource for Source {
        async fn due(&self) -> Result<Vec<ScheduledMessage>> {
            let mut block = Block::new();
            block.line("hello");
            Ok(vec![ScheduledMessage {
                id: 7,
                chat_id: 1,
                block,
            }])
        }

        async fn delivered(&self, id: i64) -> Result<()> {
            self.delivered.lock().unwrap().push(id);
            Ok(())
        }
    }

    /// Records every text message the scheduler asks to send.
    #[derive(Clone, Default)]
    struct Mock {
        sent: Arc<Mutex<Vec<String>>>,
    }

    #[crate::async_trait]
    impl Messenger for Mock {
        async fn send_text(
            &self,
            _chat: ChatId,
            text: String,
            _markup: Option<Markup>,
        ) -> ResponseResult<MessageId> {
            self.sent.lock().unwrap().push(text);
            Ok(MessageId(1))
        }

        async fn send_photo(
            &self,
            _chat: ChatId,
            _reply_to: MessageId,
            _bytes: Vec<u8>,
            _caption: Option<String>,
            _markup: Option<Markup>,
        ) -> ResponseResult<MessageId> {
            unimplemented!()
        }

        async fn edit_text(
            &self,
            _chat: ChatId,
            _msg: MessageId,
            _text: String,
        ) -> ResponseResult<()> {
            unimplemented!()
        }

        async fn delete(&self, _chat: ChatId, _msg: MessageId) -> ResponseResult<()> {
            unimplemented!()
        }

        async fn answer_callback(&self, _query_id: CallbackQueryId) -> ResponseResult<()> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn run_once_sends_and_marks_delivered() {
        let source = Arc::new(Source::default());
        let scheduler = Scheduler {
            interval: Duration::from_secs(1),
            source: source.clone(),
        };
        let messenger = Mock::default();

        scheduler.run_once(&messenger).await;

        assert_eq!(*messenger.sent.lock().unwrap(), vec!["hello".to_string()]);
        assert_eq!(*source.delivered.lock().unwrap(), vec![7]);
    }
}
