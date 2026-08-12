//! Public CoinGecko data types.

/// One trending coin.
#[derive(Debug, Clone)]
pub struct TrendingCoin {
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u32>,
    /// 24h percent change in USD.
    pub change_24h: Option<f64>,
}
