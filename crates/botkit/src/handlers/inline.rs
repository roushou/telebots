//! Inline query support: `@botname <query>` in any chat, answered with a
//! list of results.

use anyhow::Result;

use crate::request::InlineRequest;

/// One result in an inline query answer.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum InlineResult {
    /// A text card; tapping it sends `message` into the chat.
    Article {
        /// Unique id among this answer's results (1-64 bytes).
        id: String,
        /// The bold headline.
        title: String,
        /// The subtitle line, when any.
        description: Option<String>,
        /// The text sent when the card is tapped.
        message: String,
    },
}

impl InlineResult {
    /// Convert to the teloxide result type this maps onto.
    pub(crate) fn into_telegram(self) -> teloxide::types::InlineQueryResult {
        let InlineResult::Article {
            id,
            title,
            description,
            message,
        } = self;
        let content =
            teloxide::types::InputMessageContent::Text(teloxide::types::InputMessageContentText {
                message_text: message,
                parse_mode: None,
                entities: None,
                link_preview_options: None,
            });
        let mut article = teloxide::types::InlineQueryResultArticle::new(id, title, content);
        if let Some(description) = description {
            article = article.description(description);
        }
        teloxide::types::InlineQueryResult::Article(article)
    }
}

/// The outcome of an inline query: the results to show.
#[derive(Debug, Clone, Default)]
pub struct InlineAnswer {
    pub results: Vec<InlineResult>,
}

impl InlineAnswer {
    /// An empty answer (no results shown).
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one result.
    pub fn push(&mut self, result: InlineResult) -> &mut Self {
        self.results.push(result);
        self
    }

    /// Append one result, consuming the answer (for builder-style chains).
    pub fn with(mut self, result: InlineResult) -> Self {
        self.results.push(result);
        self
    }
}

/// The behavior a bot implements for `@botname` inline queries.
#[crate::async_trait]
pub trait InlineHandler: Clone + Send + Sync + 'static {
    /// Everything the handler needs to produce its results.
    type Ctx: Clone + Send + Sync + 'static;

    /// Produce the results for this query.
    ///
    /// Errors are authored with `anyhow`; botkit logs them and answers with
    /// an empty result list.
    async fn handle(&self, ctx: &Self::Ctx, req: &InlineRequest) -> Result<InlineAnswer>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn article_converts_to_telegram_result() {
        let result = InlineResult::Article {
            id: "btc".into(),
            title: "Bitcoin (BTC)".into(),
            description: Some("Price: $95,432.1".into()),
            message: "card".into(),
        }
        .into_telegram();

        match result {
            teloxide::types::InlineQueryResult::Article(article) => {
                assert_eq!(article.id, "btc");
                assert_eq!(article.title, "Bitcoin (BTC)");
                assert_eq!(article.description.as_deref(), Some("Price: $95,432.1"));
                match &article.input_message_content {
                    teloxide::types::InputMessageContent::Text(text) => {
                        assert_eq!(text.message_text, "card");
                    }
                    other => panic!("expected text content, got {other:?}"),
                }
            }
            other => panic!("expected article, got {other:?}"),
        }
    }

    #[test]
    fn empty_answer_is_default() {
        assert!(InlineAnswer::new().results.is_empty());
        assert!(InlineAnswer::default().results.is_empty());
    }
}
