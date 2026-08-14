//! The command context: what a command knows about the update that invoked
//! it. A teloxide-free seam over the transport's message type.

/// Everything a command needs about the update it is handling.
#[derive(Clone, Debug)]
pub struct Request {
    /// Telegram chat id the update came from.
    pub chat_id: i64,
    /// Telegram user id, when the update is from a private user.
    pub user_id: Option<i64>,
}

impl Request {
    /// A request for tests and callers that don't have a real update.
    pub fn new(chat_id: i64, user_id: Option<i64>) -> Self {
        Self { chat_id, user_id }
    }

    pub(crate) fn from_message(msg: &teloxide::types::Message) -> Self {
        Self {
            chat_id: msg.chat.id.0,
            user_id: msg.from.as_ref().map(|user| user.id.0 as i64),
        }
    }
}
