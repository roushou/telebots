//! Inline-keyboard button handling for degen.

use anyhow::{Result, bail};
use botkit::{CallbackHandler, CallbackRequest, Reply};
use telebots_core::Block;

use crate::{
    commands::{Ctx, Symbols},
    render,
};

/// Handles the `/price` "Refresh" button (callback data `price:btc eth`):
/// re-fetches and edits the message in place.
#[derive(Clone)]
pub struct PriceRefresh;

#[botkit::async_trait]
impl CallbackHandler for PriceRefresh {
    type Ctx = Ctx;

    async fn handle(&self, ctx: &Ctx, req: &CallbackRequest) -> Result<Reply> {
        let symbols = parse_symbols(&req.data)?;
        let quotes = ctx.cmc.quotes(&symbols).await?;

        let mut block = Block::new();
        for (i, quote) in quotes.iter().enumerate() {
            if i > 0 {
                block.blank();
            }
            block.push_block(render::quote_card(quote));
        }
        Ok(Reply::Edit(block))
    }
}

/// Parse `price:<symbols>` button data into symbols.
fn parse_symbols(data: &str) -> Result<Symbols> {
    let Some(symbols) = data.strip_prefix("price:") else {
        bail!("unknown button");
    };
    let symbols = Symbols::parse(symbols);
    if symbols.is_empty() {
        bail!("empty refresh");
    }
    Ok(symbols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_price_refresh_data() -> anyhow::Result<()> {
        let symbols = parse_symbols("price:btc eth")?;
        assert_eq!(&*symbols, &["BTC".to_string(), "ETH".to_string()]);
        Ok(())
    }

    #[test]
    fn rejects_unknown_data() {
        assert!(parse_symbols("nope:btc").is_err());
    }

    #[test]
    fn rejects_empty_symbols() {
        assert!(parse_symbols("price:").is_err());
        assert!(parse_symbols("price:   ").is_err());
    }
}
