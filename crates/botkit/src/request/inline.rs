//! The inline-query request type.

/// A teloxide-free view of an inline query.
#[derive(Debug, Clone)]
pub struct InlineRequest {
    /// The query text (everything after `@botname `).
    pub query: String,
    /// The user who issued the query.
    pub user_id: Option<i64>,
    /// The user's `@username`, when known.
    pub username: Option<String>,
}

impl InlineRequest {
    /// A request for tests and callers that don't have a real query.
    pub fn new(query: impl Into<String>, user_id: Option<i64>) -> Self {
        Self {
            query: query.into(),
            user_id,
            username: None,
        }
    }

    pub(crate) fn from_query(query: &teloxide::types::InlineQuery) -> Self {
        Self {
            query: query.query.clone(),
            user_id: Some(query.from.id.0 as i64),
            username: query.from.username.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_request_new() {
        let req = InlineRequest::new("btc eth", Some(42));
        assert_eq!(req.query, "btc eth");
        assert_eq!(req.user_id, Some(42));
        assert_eq!(req.username, None);
    }
}
