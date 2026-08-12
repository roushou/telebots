//! Public CoinMarketCap data types.

/// A CoinMarketCap quote.
#[derive(Debug, Clone)]
pub struct Quote {
    pub id: Option<u64>,
    pub name: String,
    pub symbol: String,
    pub rank: Option<u32>,
    pub price: Option<f64>,
    pub change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
}

/// Global market metrics (from `global-metrics/quotes/latest`).
#[derive(Debug, Clone)]
pub struct GlobalMetrics {
    pub total_market_cap: f64,
    pub change_24h: Option<f64>,
    pub btc_dominance: f64,
    pub eth_dominance: f64,
}

/// Project metadata (from `cryptocurrency/info`).
#[derive(Debug, Clone)]
pub struct CoinInfo {
    pub name: String,
    pub symbol: String,
    pub category: String,
    pub website: Option<String>,
    pub description: String,
}

/// The Fear & Greed index, 0 (extreme fear) to 100 (extreme greed).
/// From CMC's keyless public API (`/public-api/v3/fear-and-greed/latest`).
#[derive(Debug, Clone)]
pub struct FearGreed {
    pub value: u8,
    pub classification: String,
}
