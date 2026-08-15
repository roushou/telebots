//! The text generator: a provider abstraction.
//!
//! Bots depend on this enum, never on a provider crate directly. Each
//! variant wraps a provider client; [`Generator::chat`] normalizes the
//! provider's output to plain text, so swapping providers only touches this
//! module and the `Ctx` wiring.

use anyhow::Result;
use cloudflare_ai::{ChatMessage, CloudflareAiClient, TextModel};

/// The configured text-generation provider.
#[derive(Clone)]
pub enum Generator {
    Cloudflare(CloudflareAiClient),
}

impl Generator {
    pub fn cloudflare(account_id: String, api_token: String) -> Result<Self> {
        Ok(Self::Cloudflare(CloudflareAiClient::new(
            account_id, api_token,
        )?))
    }

    /// Complete the conversation with `model`, returning the assistant reply.
    pub async fn chat(&self, model: TextModel, messages: &[ChatMessage]) -> Result<String> {
        match self {
            Generator::Cloudflare(client) => Ok(client.chat(model, messages).await?.text),
        }
    }
}
