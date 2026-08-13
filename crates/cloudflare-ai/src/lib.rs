//! Cloudflare Workers AI client — image generation (free tier).
//!
//! [`CloudflareAiClient`] performs the requests; the generated image type
//! lives in [`types`].

mod client;
mod error;
mod types;

pub use client::CloudflareAiClient;
pub use error::Error;
pub use types::{GeneratedImage, Input, Model};
