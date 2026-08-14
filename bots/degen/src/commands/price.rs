//! `/price` — latest quotes for one or more symbols.

use anyhow::{Result, bail};
use botkit::{Button, Markup, Reply};
use telebots_core::Block;

use crate::{
    commands::{Ctx, args::Symbols},
    render,
};

/// Typed arguments for `/price`.
#[derive(Debug, Clone)]
pub struct PriceArgs {
    pub symbols: Symbols,
}

impl PriceArgs {
    /// Parse and validate the raw argument string.
    pub fn parse(raw: &str) -> Result<Self> {
        let symbols = Symbols::parse(raw);
        if symbols.is_empty() {
            bail!("Usage: /price btc eth sol");
        }
        Ok(Self { symbols })
    }

    /// Produce the reply: one card per quote, blank-line separated, with a
    /// "Refresh" button that re-runs this query in place.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Reply> {
        let quotes = ctx.cmc.quotes(&self.symbols).await?;

        let mut b = Block::new();
        for (i, q) in quotes.iter().enumerate() {
            if i > 0 {
                b.blank();
            }
            b.push_block(render::quote_card(q));
        }
        Ok(Reply::text(b).with_markup(self.refresh_markup()))
    }

    /// The keyboard: a "Refresh" button carrying `price:<symbols>`.
    fn refresh_markup(&self) -> Markup {
        Markup::new().row([Button::callback(
            "Refresh",
            format!("price:{}", self.symbols.join(" ")),
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_symbols() {
        assert!(PriceArgs::parse("").is_err());
        assert!(PriceArgs::parse("   ").is_err());
    }

    #[test]
    fn parse_uppercases_symbols() -> anyhow::Result<()> {
        let args = PriceArgs::parse("btc eth")?;
        assert_eq!(&*args.symbols, &["BTC".to_string(), "ETH".to_string()]);
        Ok(())
    }
}
