//! CoinMarketCap API client (free tier endpoints only).
//!
//! [`CmcClient`] performs the requests; the returned data types
//! ([`Quote`], [`GlobalMetrics`], [`CoinInfo`], [`FearGreed`]) live in
//! [`types`] and render themselves as text blocks.

mod client;
mod types;

pub use client::CmcClient;
pub use types::{CoinInfo, FearGreed, GlobalMetrics, Quote};
