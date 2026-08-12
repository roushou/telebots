//! The crate's error type.

use thiserror::Error;

/// Errors returned by the CoinMarketCap client.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP request, status, or body could not be handled. The wrapped
    /// [`reqwest::Error`] distinguishes transport, status, and decode
    /// failures (`status()`, `is_timeout()`, …).
    #[error("CoinMarketCap request failed")]
    Http(#[from] reqwest::Error),

    /// CoinMarketCap returned an API-level error in its status envelope.
    #[error("CoinMarketCap error {code}: {message}")]
    Api { code: i32, message: String },

    /// A required field was absent from an otherwise-successful response.
    #[error("missing {what}")]
    MissingData { what: String },

    /// No symbols were supplied to an endpoint that requires them.
    #[error("no symbols provided")]
    NoSymbols,
}
