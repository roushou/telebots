//! The callback-query request type.

/// A teloxide-free view of a button tap.
#[derive(Debug, Clone)]
pub struct CallbackRequest {
    /// The button's callback data.
    pub data: String,
    /// The chat the button's message lives in, when any.
    pub chat_id: Option<i64>,
    /// The user who tapped.
    pub user_id: Option<i64>,
    /// The message the button was on, when accessible (for editing).
    pub message_id: Option<i32>,
}

impl CallbackRequest {
    /// A request for tests and callers that don't have a real query.
    pub fn new(data: impl Into<String>, chat_id: Option<i64>, message_id: Option<i32>) -> Self {
        Self {
            data: data.into(),
            chat_id,
            user_id: None,
            message_id,
        }
    }

    pub(crate) fn from_query(query: &teloxide::types::CallbackQuery) -> Self {
        let (chat_id, message_id) = match &query.message {
            Some(message) => (Some(message.chat().id.0), Some(message.id().0)),
            None => (None, None),
        };
        Self {
            data: query.data.clone().unwrap_or_default(),
            chat_id,
            user_id: Some(query.from.id.0 as i64),
            message_id,
        }
    }
}
