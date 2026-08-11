//! `/trending` — top trending coins (CoinGecko).

use anyhow::Result;
use coingecko::TrendingCoin;
use telebots_core::{Block, RenderBlock};

use crate::commands::Ctx;

const TRENDING_LIMIT: usize = 5;

/// The `/trending` command.
pub struct Trending;

impl Trending {
    fn list_block(coins: &[TrendingCoin]) -> Block {
        let mut b = Block::new();
        b.line("🔥 Trending");
        for (i, coin) in coins.iter().enumerate() {
            b.line(format!("{}. {}", i + 1, coin.to_block().build()));
        }
        b
    }

    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        let coins = ctx.coingecko.trending(TRENDING_LIMIT).await?;
        Ok(Self::list_block(&coins))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_numbered_list() {
        let coins = [
            TrendingCoin {
                name: "Velvet".into(),
                symbol: "VELVET".into(),
                market_cap_rank: Some(124),
                change_24h: Some(58.6),
            },
            TrendingCoin {
                name: "Pons".into(),
                symbol: "PONS".into(),
                market_cap_rank: None,
                change_24h: None,
            },
        ];
        assert_eq!(
            Trending::list_block(&coins).build(),
            "🔥 Trending\n1. #124 Velvet (VELVET) ▲ +58.6%\n2. Pons (PONS)"
        );
    }
}
