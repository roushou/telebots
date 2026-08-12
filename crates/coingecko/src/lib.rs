//! CoinGecko client (free, no API key).
//!
//! CMC's `trending/latest` is paid, so `/trending` uses CoinGecko's
//! `search/trending`, which needs no key.

mod client;
mod error;
mod types;

pub use client::CoinGeckoClient;
pub use error::Error;
pub use types::TrendingCoin;
