//! Cloudflare Workers AI client — image generation and text generation.
//!
//! [`CloudflareAiClient`] performs the requests; the generated-image and
//! chat types live in [`types`].

mod client;
mod error;
mod types;

pub use client::CloudflareAiClient;
pub use error::Error;
pub use types::{
    ChatCompletion, ChatMessage, GeneratedImage, ImageModel, Input, Role, TextModel, Usage,
};
