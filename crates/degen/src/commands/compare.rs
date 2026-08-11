//! `/compare` — side-by-side quotes for two or more symbols.

use anyhow::{Result, bail};
use telebots_core::Block;

use crate::{
    cmc::Quote,
    commands::{Ctx, args::Symbols},
};

/// Typed arguments for `/compare`.
#[derive(Debug, Clone)]
pub struct CompareArgs {
    pub symbols: Symbols,
}

impl CompareArgs {
    /// Parse and validate the raw argument string (needs at least two).
    pub fn parse(raw: &str) -> Result<Self> {
        let symbols = Symbols::parse(raw);
        if symbols.len() < 2 {
            bail!("Usage: /compare btc eth sol");
        }
        Ok(Self { symbols })
    }

    fn table_block(&self, quotes: &[Quote]) -> Block {
        let mut b = Block::new();
        b.line(format!("📊 {}", self.symbols.join(" vs ")));
        for q in quotes {
            b.row(q.compare_row());
        }
        b
    }

    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        let quotes = ctx.cmc.quotes(&self.symbols).await?;
        Ok(self.table_block(&quotes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_two_symbols() {
        assert!(CompareArgs::parse("").is_err());
        assert!(CompareArgs::parse("btc").is_err());
    }

    #[test]
    fn parse_accepts_two_or_more() {
        assert!(CompareArgs::parse("btc eth").is_ok());
        assert!(CompareArgs::parse("btc eth sol").is_ok());
    }

    #[test]
    fn table_block_aligns_columns() {
        let args = CompareArgs::parse("btc eth").unwrap();
        let quotes = [
            crate::cmc::Quote {
                id: Some(1),
                name: "Bitcoin".into(),
                symbol: "BTC".into(),
                rank: None,
                price: Some(95_432.1),
                change_24h: Some(1.23),
                market_cap: None,
                volume_24h: None,
            },
            crate::cmc::Quote {
                id: Some(1027),
                name: "Ethereum".into(),
                symbol: "ETH".into(),
                rank: None,
                price: Some(3_500.0),
                change_24h: Some(-0.5),
                market_cap: None,
                volume_24h: None,
            },
        ];
        assert_eq!(
            args.table_block(&quotes).build(),
            "📊 BTC vs ETH\nBTC  $95,432.1  ▲ +1.23%\nETH     $3,500  ▼ -0.50%"
        );
    }
}
