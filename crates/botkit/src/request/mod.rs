//! The teloxide-free views of the updates handlers receive: [`Request`] for
//! commands and the message, inline, and callback request types for their
//! handlers.

mod callback;
mod inline;
mod message;

pub use callback::CallbackRequest;
pub use inline::InlineRequest;
pub use message::MessageRequest;

/// The kind of chat a request came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChatKind {
    Private,
    Group,
    Supergroup,
    Channel,
}

/// Everything a handler knows about the update it is handling.
#[derive(Debug, Clone)]
pub struct Request {
    /// Telegram chat id the update came from.
    pub chat_id: i64,
    /// Telegram user id, when the update is from a private user.
    pub user_id: Option<i64>,
    /// The user's `@username`, when known.
    pub username: Option<String>,
    /// What kind of chat this is.
    pub chat_kind: ChatKind,
    /// The message being replied to, when any.
    pub reply_to_message_id: Option<i32>,
}

impl Request {
    /// A request for tests and callers that don't have a real update.
    pub fn new(chat_id: i64, user_id: Option<i64>) -> Self {
        Self {
            chat_id,
            user_id,
            username: None,
            chat_kind: ChatKind::Private,
            reply_to_message_id: None,
        }
    }

    pub(crate) fn from_message(msg: &teloxide::types::Message) -> Self {
        Self {
            chat_id: msg.chat.id.0,
            user_id: msg.from.as_ref().map(|user| user.id.0 as i64),
            username: msg.from.as_ref().and_then(|user| user.username.clone()),
            chat_kind: chat_kind(&msg.chat.kind),
            reply_to_message_id: msg.reply_to_message().map(|reply| reply.id.0),
        }
    }
}

/// Map a Telegram chat kind to the teloxide-free [`ChatKind`].
pub(crate) fn chat_kind(kind: &teloxide::types::ChatKind) -> ChatKind {
    match kind {
        teloxide::types::ChatKind::Private(_) => ChatKind::Private,
        teloxide::types::ChatKind::Public(public) => match &public.kind {
            teloxide::types::PublicChatKind::Group => ChatKind::Group,
            teloxide::types::PublicChatKind::Supergroup(_) => ChatKind::Supergroup,
            teloxide::types::PublicChatKind::Channel(_) => ChatKind::Channel,
        },
    }
}
