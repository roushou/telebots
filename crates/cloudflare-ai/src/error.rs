//! The crate's error type.

use thiserror::Error;

/// Errors returned by the Cloudflare Workers AI client.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The HTTP request could not be sent or read. The wrapped
    /// [`reqwest::Error`] distinguishes transport, status, and decode
    /// failures (`status()`, `is_timeout()`, …).
    #[error("Cloudflare request failed")]
    Http(#[from] reqwest::Error),

    /// Cloudflare answered with a non-success HTTP status.
    #[error("Cloudflare error {status}: {detail}")]
    Api { status: u16, detail: String },

    /// The response envelope carried `"success": false`.
    #[error("Cloudflare request not successful: {detail}")]
    NotSuccess { detail: String },

    /// The body was neither raw image bytes nor the JSON envelope.
    #[error("Cloudflare response was neither an image nor the JSON envelope")]
    UnexpectedPayload(#[from] serde_json::Error),

    /// A successful envelope did not contain `result.image`.
    #[error("Cloudflare success response missing result.image")]
    MissingImage,

    /// A successful text-generation envelope did not contain
    /// `result.response`.
    #[error("Cloudflare success response missing result.response")]
    MissingResponse,

    /// `result.image` was not valid base64.
    #[error("result.image was not valid base64")]
    InvalidBase64(#[from] base64::DecodeError),

    /// A model name was not recognized (see [`crate::Model::from_str`] and
    /// [`crate::TextModel::from_str`]).
    #[error("unknown model: {0}")]
    InvalidModel(String),
}
