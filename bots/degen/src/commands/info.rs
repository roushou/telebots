//! `/info` — project metadata (category, website, description) via CMC.

use anyhow::{Result, bail};
use telebots_core::Block;

use crate::{
    commands::{Ctx, args::Symbols},
    render,
};

/// Typed arguments for `/info`.
#[derive(Debug, Clone)]
pub struct InfoArgs {
    pub symbols: Symbols,
}

impl InfoArgs {
    /// Parse and validate the raw argument string.
    pub fn parse(raw: &str) -> Result<Self> {
        let symbols = Symbols::parse(raw);
        if symbols.is_empty() {
            bail!("Usage: /info btc eth");
        }
        Ok(Self { symbols })
    }

    /// Produce the reply block: one card per coin, blank-line separated.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        let infos = ctx.cmc.info(&self.symbols).await?;

        let mut b = Block::new();
        for (i, info) in infos.iter().enumerate() {
            if i > 0 {
                b.blank();
            }
            b.push_block(render::coin_info_card(info));
        }
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_symbols() {
        assert!(InfoArgs::parse("").is_err());
    }

    #[test]
    fn parse_uppercases_symbols() -> anyhow::Result<()> {
        let args = InfoArgs::parse("btc eth")?;
        assert_eq!(&*args.symbols, &["BTC".to_string(), "ETH".to_string()]);
        Ok(())
    }
}
