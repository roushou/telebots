//! Inline query handling: `@degen btc eth` shows price cards, tappable from
//! any chat without adding the bot.

use anyhow::Result;
use botkit::{InlineAnswer, InlineHandler, InlineRequest, InlineResult};

use crate::{commands::Symbols, render};

/// The inline query handler.
#[derive(Clone)]
pub struct Inline;

#[botkit::async_trait]
impl InlineHandler for Inline {
    type Ctx = crate::commands::Ctx;

    async fn handle(&self, ctx: &Self::Ctx, req: &InlineRequest) -> Result<InlineAnswer> {
        let symbols = Symbols::parse(&req.query);
        if symbols.is_empty() {
            return Ok(InlineAnswer::new());
        }

        let quotes = ctx.cmc.quotes(&symbols).await?;
        let mut answer = InlineAnswer::new();
        for quote in &quotes {
            answer.push(InlineResult::Article {
                id: quote.symbol.clone(),
                title: format!("{} ({})", quote.name, quote.symbol),
                description: Some(render::quote_summary(quote)),
                message: render::quote_card(quote).build(),
            });
        }
        Ok(answer)
    }
}
