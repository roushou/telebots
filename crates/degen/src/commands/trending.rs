//! `/trending` — top trending coins (CoinGecko).

use anyhow::Result;
use telebots_core::{Block, RenderBlock};
use teloxide::prelude::*;

use crate::{coingecko::CoinGeckoClient, commands::util};

const TRENDING_LIMIT: usize = 5;

pub async fn handle(bot: Bot, msg: Message, client: CoinGeckoClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&client).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(client: &CoinGeckoClient) -> Result<String> {
    let coins = client.trending(TRENDING_LIMIT).await?;

    let mut b = Block::new();
    b.line("🔥 Trending");
    for (i, coin) in coins.iter().enumerate() {
        b.line(format!("{}. {}", i + 1, coin.to_block().build()));
    }
    Ok(b.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coingecko::TrendingCoin;

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
        let mut b = Block::new();
        b.line("🔥 Trending");
        for (i, coin) in coins.iter().enumerate() {
            b.line(format!("{}. {}", i + 1, coin.to_block().build()));
        }
        assert_eq!(
            b.build(),
            "🔥 Trending\n1. #124 Velvet (VELVET) ▲ +58.6%\n2. Pons (PONS)"
        );
    }
}
