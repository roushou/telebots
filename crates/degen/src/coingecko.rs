//! CoinGecko client — trending coins (free, no API key).
//!
//! CMC's `trending/latest` is paid, so `/trending` uses CoinGecko's
//! `search/trending`, which needs no key.

use anyhow::{Context, Result};
use serde::Deserialize;
use telebots_core::{Block, Change, RenderBlock};

const API_BASE: &str = "https://api.coingecko.com/api/v3";

/// One trending coin.
#[derive(Debug, Clone)]
pub struct TrendingCoin {
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u32>,
    /// 24h percent change in USD.
    pub change_24h: Option<f64>,
}

impl RenderBlock for TrendingCoin {
    fn render_block(&self, out: &mut Block) {
        let rank = self
            .market_cap_rank
            .map(|r| format!("#{r} "))
            .unwrap_or_default();
        let change = self
            .change_24h
            .map(|c| format!(" {}", Change::new(c).with_decimals(1)))
            .unwrap_or_default();
        out.line(format!("{rank}{} ({}){change}", self.name, self.symbol));
    }
}

#[derive(Clone)]
pub struct CoinGeckoClient {
    http: reqwest::Client,
}

impl CoinGeckoClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            // CoinGecko asks clients to identify themselves.
            .user_agent("degen-bot/0.1")
            .build()
            .expect("failed to build HTTP client");
        Self { http }
    }

    /// The `limit` most-trending coins right now.
    pub async fn trending(&self, limit: usize) -> Result<Vec<TrendingCoin>> {
        let resp: TrendingResponse = self
            .http
            .get(format!("{API_BASE}/search/trending"))
            .send()
            .await
            .context("CoinGecko request failed")?
            .error_for_status()
            .context("CoinGecko returned an error status")?
            .json()
            .await
            .context("failed to parse CoinGecko response")?;

        Ok(resp
            .coins
            .into_iter()
            .take(limit)
            .map(|c| TrendingCoin {
                name: c.item.name,
                symbol: c.item.symbol.to_uppercase(),
                market_cap_rank: c.item.market_cap_rank,
                change_24h: c.item.data.price_change_percentage_24h.and_then(|m| m.usd),
            })
            .collect())
    }
}

impl Default for CoinGeckoClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TrendingResponse {
    coins: Vec<TrendingEntry>,
}

#[derive(Deserialize)]
struct TrendingEntry {
    item: TrendingItem,
}

#[derive(Deserialize)]
struct TrendingItem {
    name: String,
    symbol: String,
    market_cap_rank: Option<u32>,
    data: TrendingData,
}

#[derive(Deserialize)]
struct TrendingData {
    price_change_percentage_24h: Option<PriceChange>,
}

#[derive(Deserialize)]
struct PriceChange {
    usd: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trending_entry_block() {
        let coin = TrendingCoin {
            name: "Velvet".into(),
            symbol: "VELVET".into(),
            market_cap_rank: Some(124),
            change_24h: Some(58.6),
        };
        assert_eq!(coin.to_block().build(), "#124 Velvet (VELVET) ▲ +58.6%");
    }

    #[test]
    fn trending_entry_without_rank_or_change() {
        let coin = TrendingCoin {
            name: "Pons".into(),
            symbol: "PONS".into(),
            market_cap_rank: None,
            change_24h: None,
        };
        assert_eq!(coin.to_block().build(), "Pons (PONS)");
    }
}
