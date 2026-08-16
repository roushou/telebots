//! Text generation: request plumbing and response decoding.

use std::time::Instant;

use serde_json::{Value, json};

use crate::{
    client::{API_BASE, CloudflareAiClient},
    error::Error,
    types::{ChatCompletion, ChatMessage, TextModel, Usage},
};

impl CloudflareAiClient {
    /// Run a text-generation (chat) request with the given `messages` using
    /// `model`, returning the assistant's reply. Requests use Cloudflare's
    /// recommended scoped-prompt format (`{"messages":[…]}`).
    pub async fn chat(
        &self,
        model: TextModel,
        messages: &[ChatMessage],
    ) -> Result<ChatCompletion, Error> {
        let url = format!(
            "{API_BASE}/accounts/{}/ai/run/{}",
            self.account_id,
            model.path()
        );
        let body = json!({ "messages": messages
            .iter()
            .map(|m| json!({ "role": m.role.as_str(), "content": m.content }))
            .collect::<Vec<_>>() });
        let started = Instant::now();
        tracing::debug!(model = %model, messages = messages.len(), "sending chat request");
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.api_token)
            .json(&body)
            .send()
            .await?;
        tracing::debug!(
            status = resp.status().as_u16(),
            elapsed = ?started.elapsed(),
            "chat request completed"
        );

        let status = resp.status();
        let bytes = resp.bytes().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                detail: Self::error_detail(&bytes).unwrap_or_default(),
            });
        }

        Self::decode_chat(&bytes)
    }

    /// Decode a Workers AI text-generation response:
    ///
    /// ```json
    /// {"result":{"response":"<text>","usage":{...}},"success":true,...}
    /// ```
    fn decode_chat(bytes: &[u8]) -> Result<ChatCompletion, Error> {
        let v: Value = serde_json::from_slice(bytes)?;

        if !v.get("success").and_then(Value::as_bool).unwrap_or(false) {
            return Err(Error::NotSuccess {
                detail: Self::error_detail(bytes).unwrap_or_default(),
            });
        }

        let text = v
            .get("result")
            .and_then(|r| r.get("response"))
            .and_then(Value::as_str)
            .ok_or(Error::MissingResponse)?;

        let usage = v
            .get("result")
            .and_then(|r| r.get("usage"))
            .map(Self::parse_usage);

        Ok(ChatCompletion {
            text: text.to_string(),
            usage,
        })
    }

    /// Parse the optional `result.usage` object; absent fields default to
    /// zero and a missing `total_tokens` is derived from the parts.
    fn parse_usage(u: &Value) -> Usage {
        let prompt = u.get("prompt_tokens").and_then(Value::as_u64).unwrap_or(0);
        let completion = u
            .get("completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let total = u.get("total_tokens").and_then(Value::as_u64).unwrap_or(0);
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: if total > 0 {
                total
            } else {
                prompt + completion
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_chat_response() -> Result<(), Error> {
        let body =
            br#"{"result":{"response":"Hello, World!"},"success":true,"errors":[],"messages":[]}"#;
        let chat = CloudflareAiClient::decode_chat(body)?;
        assert_eq!(chat.text, "Hello, World!");
        assert_eq!(chat.usage, None);
        Ok(())
    }

    #[test]
    fn decodes_chat_usage() -> Result<(), Error> {
        let body = br#"{"result":{"response":"hi","usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}},"success":true,"errors":[],"messages":[]}"#;
        let chat = CloudflareAiClient::decode_chat(body)?;
        let usage = chat.usage.expect("usage present");
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
        Ok(())
    }

    #[test]
    fn derives_total_tokens_when_missing() -> Result<(), Error> {
        let body = br#"{"result":{"response":"hi","usage":{"prompt_tokens":10,"completion_tokens":20}},"success":true,"errors":[],"messages":[]}"#;
        let chat = CloudflareAiClient::decode_chat(body)?;
        assert_eq!(chat.usage.expect("usage present").total_tokens, 30);
        Ok(())
    }

    #[test]
    fn rejects_chat_with_success_false() {
        let body = br#"{"result":{},"success":false,"errors":[{"code":10000,"message":"model not found"}],"messages":[]}"#;
        let err = CloudflareAiClient::decode_chat(body).unwrap_err();
        assert!(err.to_string().contains("model not found"));
    }

    #[test]
    fn rejects_chat_missing_response() {
        let body = br#"{"result":{},"success":true,"errors":[],"messages":[]}"#;
        assert!(CloudflareAiClient::decode_chat(body).is_err());
    }
}
