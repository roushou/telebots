//! Free-form message support: the branch that lets a bot respond to any
//! text message, not just slash commands.
//!
//! A [`MessageHandler`] sees every text message that was not consumed by a
//! command branch and may reply (or return `None` to stay silent).

use anyhow::Result;

use crate::{ChatKind, Reply};

/// A teloxide-free view of a free-form text message.
#[derive(Debug, Clone)]
pub struct MessageRequest {
    /// The full message text.
    pub text: String,
    /// Telegram chat id the message came from.
    pub chat_id: i64,
    /// Telegram user id, when the message is from a private user.
    pub user_id: Option<i64>,
    /// The user's `@username`, when known.
    pub username: Option<String>,
    /// What kind of chat this is.
    pub chat_kind: ChatKind,
    /// The message being replied to, when any.
    pub reply_to_message_id: Option<i32>,
    /// Whether the text mentions the bot (contains `@botname`).
    pub mentioned: bool,
    /// Whether this message replies to one of the bot's own messages.
    pub replied_to_bot: bool,
}

impl MessageRequest {
    /// A request for tests and callers that don't have a real update.
    pub fn new(text: impl Into<String>, chat_id: i64, user_id: Option<i64>) -> Self {
        Self {
            text: text.into(),
            chat_id,
            user_id,
            username: None,
            chat_kind: ChatKind::Private,
            reply_to_message_id: None,
            mentioned: false,
            replied_to_bot: false,
        }
    }
}

/// The behavior a bot implements for free-form text messages.
#[crate::async_trait]
pub trait MessageHandler: Clone + Send + Sync + 'static {
    /// Everything the handler needs to produce its reply.
    type Ctx: Clone + Send + Sync + 'static;

    /// Produce the reply for this message, or `None` to stay silent (for
    /// example, group chatter that is neither an @mention nor a reply).
    ///
    /// Errors are authored with `anyhow`; botkit transports and renders them
    /// (`⚠️ {e:#}`).
    async fn handle(&self, ctx: &Self::Ctx, req: &MessageRequest) -> Result<Option<Reply>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_request_new() {
        let req = MessageRequest::new("hello", 1, Some(42));
        assert_eq!(req.text, "hello");
        assert_eq!(req.chat_id, 1);
        assert_eq!(req.user_id, Some(42));
        assert!(!req.mentioned);
        assert!(!req.replied_to_bot);
        assert_eq!(req.chat_kind, ChatKind::Private);
    }
}
