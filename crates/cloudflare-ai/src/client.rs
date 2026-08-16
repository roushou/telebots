//! The Cloudflare Workers AI HTTP client: the shared connection and error
//! envelope parsing. Image generation lives in [`image`], text generation in
//! [`text`].

use serde_json::Value;

use crate::error::Error;

/// Base URL for the Cloudflare REST API.
pub(crate) const API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// A Cloudflare Workers AI client over one account.
#[derive(Clone)]
pub struct CloudflareAiClient {
    pub(crate) http: reqwest::Client,
    pub(crate) account_id: String,
    pub(crate) api_token: String,
}

impl CloudflareAiClient {
    pub fn new(account_id: String, api_token: String) -> Result<Self, Error> {
        let http = reqwest::Client::builder()
            .user_agent(format!(
                "{}/{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self {
            http,
            account_id,
            api_token,
        })
    }

    /// Pull the `errors[0].message` from a Cloudflare JSON error envelope.
    pub(crate) fn error_detail(bytes: &[u8]) -> Option<String> {
        let v: Value = serde_json::from_slice(bytes).ok()?;
        let msg = v
            .get("errors")
            .and_then(|e| e.as_array())
            .and_then(|a| a.first())
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())?;
        Some(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_message_from_error_envelope() {
        let body = br#"{"errors":[{"code":10000,"message":"model not found"}],"success":false}"#;
        assert_eq!(
            CloudflareAiClient::error_detail(body).as_deref(),
            Some("model not found")
        );
    }

    #[test]
    fn no_message_when_envelope_is_irregular() {
        assert_eq!(CloudflareAiClient::error_detail(b"not json"), None);
    }
}
