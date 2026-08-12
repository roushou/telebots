//! CoinMarketCap API client (free tier endpoints only).
//!
//! [`CmcClient`] performs the requests; the returned data types
//! ([`Quote`], [`GlobalMetrics`], [`CoinInfo`], [`FearGreed`]) live in
//! [`types`].

mod client;
mod error;
mod types;

pub use client::CmcClient;
pub use error::Error;
pub use types::{CoinInfo, FearGreed, GlobalMetrics, Quote};
