//! `/price` — latest quotes for one or more symbols.

use anyhow::{Result, bail};
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

    /// Produce the reply block: one card per quote, blank-line separated.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        let quotes = ctx.cmc.quotes(&self.symbols).await?;

        let mut b = Block::new();
        for (i, q) in quotes.iter().enumerate() {
            if i > 0 {
                b.blank();
            }
            b.push_block(render::quote_card(q));
        }
        Ok(b)
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
    fn parse_uppercases_symbols() {
        let args = PriceArgs::parse("btc eth").unwrap();
        assert_eq!(&*args.symbols, &["BTC".to_string(), "ETH".to_string()]);
    }
}
