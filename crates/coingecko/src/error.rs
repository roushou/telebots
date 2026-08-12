//! The crate's error type.

use thiserror::Error;

/// Errors returned by the CoinGecko client.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP request, status, or body could not be handled. The wrapped
    /// [`reqwest::Error`] distinguishes transport, status, and decode
    /// failures (`status()`, `is_timeout()`, …).
    #[error("CoinGecko request failed")]
    Http(#[from] reqwest::Error),
}
