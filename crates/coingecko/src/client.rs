//! The CoinGecko HTTP client and its wire-format response types. The public
//! data type lives in [`types`].

use serde::Deserialize;

use crate::{error::Error, types::TrendingCoin};

const API_BASE: &str = "https://api.coingecko.com/api/v3";

#[derive(Clone)]
pub struct CoinGeckoClient {
    http: reqwest::Client,
}

impl CoinGeckoClient {
    pub fn new() -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            // CoinGecko asks clients to identify themselves.
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self { http })
    }

    /// The `limit` most-trending coins right now.
    pub async fn trending(&self, limit: usize) -> Result<Vec<TrendingCoin>, Error> {
        let resp: TrendingResponse = self
            .http
            .get(format!("{API_BASE}/search/trending"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

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

// --- Wire-format response types -----------------------------------------

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
