//! The free-form message request type.

use crate::request::Request;

/// A teloxide-free view of a free-form text message: the shared [`Request`]
/// context plus the message-specific fields.
#[derive(Debug, Clone)]
pub struct MessageRequest {
    /// The shared update context (chat, user, chat kind, reply target).
    pub request: Request,
    /// The full message text.
    pub text: String,
    /// Whether the text mentions the bot (contains `@botname`).
    pub mentioned: bool,
    /// Whether this message replies to one of the bot's own messages.
    pub replied_to_bot: bool,
}

impl MessageRequest {
    /// A request for tests and callers that don't have a real update.
    pub fn new(text: impl Into<String>, chat_id: i64, user_id: Option<i64>) -> Self {
        Self {
            request: Request::new(chat_id, user_id),
            text: text.into(),
            mentioned: false,
            replied_to_bot: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::ChatKind;

    #[test]
    fn message_request_new() {
        let req = MessageRequest::new("hello", 1, Some(42));
        assert_eq!(req.text, "hello");
        assert_eq!(req.request.chat_id, 1);
        assert_eq!(req.request.user_id, Some(42));
        assert!(!req.mentioned);
        assert!(!req.replied_to_bot);
        assert_eq!(req.request.chat_kind, ChatKind::Private);
    }
}
