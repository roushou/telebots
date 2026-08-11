//! `/trending` — top trending coins (CoinGecko).

use anyhow::Result;
use teloxide::prelude::*;

use crate::{coingecko::CoinGeckoClient, commands::util};

const TRENDING_LIMIT: usize = 5;

pub async fn handle(bot: Bot, msg: Message, client: CoinGeckoClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&client).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(client: &CoinGeckoClient) -> Result<String> {
    Ok(format_list(&client.trending(TRENDING_LIMIT).await?))
}

pub fn format_list(coins: &[crate::coingecko::TrendingCoin]) -> String {
    let mut out = String::from("🔥 Trending");
    for (i, c) in coins.iter().enumerate() {
        let rank = c
            .market_cap_rank
            .map(|r| format!("#{r} "))
            .unwrap_or_default();
        let change = c
            .change_24h
            .map(|c| {
                let arrow = if c >= 0.0 { "▲" } else { "▼" };
                format!(" {arrow} {c:+.1}%")
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "\n{}. {rank}{} ({}){change}",
            i + 1,
            c.name,
            c.symbol
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coingecko::TrendingCoin;

    #[test]
    fn format_ranking() {
        let coins = vec![
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
            format_list(&coins),
            "🔥 Trending\n1. #124 Velvet (VELVET) ▲ +58.6%\n2. Pons (PONS)"
        );
    }
}
