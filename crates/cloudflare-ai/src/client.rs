//! The Cloudflare Workers AI HTTP client: request plumbing and the REST
//! call. The generated-image type lives in [`types`].

use std::time::Instant;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{header::CONTENT_TYPE, multipart::Form};
use serde_json::{Value, json};

use crate::{
    error::Error,
    types::{GeneratedImage, Input, Model},
};

const API_BASE: &str = "https://api.cloudflare.com/client/v4";

#[derive(Clone)]
pub struct CloudflareAiClient {
    http: reqwest::Client,
    account_id: String,
    api_token: String,
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

    /// Generate an image from `prompt` using `model`.
    ///
    /// The request encoding is chosen by the model: JSON for the classic
    /// text-to-image models, `multipart/form-data` for the FLUX.2 family.
    pub async fn generate_image(
        &self,
        model: Model,
        prompt: &str,
    ) -> Result<GeneratedImage, Error> {
        let url = format!(
            "{API_BASE}/accounts/{}/ai/run/{}",
            self.account_id,
            model.path()
        );
        let request = self.http.post(url).bearer_auth(&self.api_token);
        let started = Instant::now();
        tracing::debug!(model = %model, "sending image request");
        let resp = match model.input() {
            Input::Json => request.json(&json!({ "prompt": prompt })).send().await?,
            Input::Multipart => {
                request
                    .multipart(Form::new().text("prompt", prompt.to_string()))
                    .send()
                    .await?
            }
        };
        tracing::debug!(
            status = resp.status().as_u16(),
            elapsed = ?started.elapsed(),
            "image request completed"
        );

        let status = resp.status();
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let bytes = resp.bytes().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                detail: Self::error_detail(&bytes).unwrap_or_default(),
            });
        }

        Self::decode_response(&content_type, &bytes)
    }

    /// Decode a Workers AI image-generation response.
    ///
    /// Image models answer with raw image bytes when the request accepts
    /// `image/*`, and with a JSON envelope otherwise:
    ///
    /// ```json
    /// {"result":{"image":"<base64>"},"success":true,"errors":[],"messages":[]}
    /// ```
    ///
    /// `result.image` is base64-encoded, sometimes wrapped in a
    /// `data:image/<mime>;base64,` data URI.
    fn decode_response(content_type: &str, bytes: &[u8]) -> Result<GeneratedImage, Error> {
        // Raw binary path: the server honors `Accept: image/*`.
        if content_type.starts_with("image/") {
            return Ok(GeneratedImage {
                bytes: bytes.to_vec(),
                mime: content_type.to_string(),
            });
        }

        // JSON envelope path.
        let v: Value = serde_json::from_slice(bytes)?;

        if !v.get("success").and_then(Value::as_bool).unwrap_or(false) {
            return Err(Error::NotSuccess {
                detail: Self::error_detail(bytes).unwrap_or_default(),
            });
        }

        let image = v
            .get("result")
            .and_then(|r| r.get("image"))
            .and_then(Value::as_str)
            .ok_or(Error::MissingImage)?;

        // Strip a `data:image/<mime>;base64,` prefix so only the base64
        // payload reaches the decoder.
        let (mime, payload) = match image.split_once(',') {
            Some((prefix, payload)) if prefix.starts_with("data:") => {
                let mime = prefix
                    .strip_prefix("data:")
                    .and_then(|p| p.split(';').next())
                    .filter(|m| !m.is_empty())
                    .unwrap_or("image/png");
                (mime.to_string(), payload)
            }
            _ => ("image/png".to_string(), image),
        };

        let bytes = STANDARD.decode(payload)?;

        Ok(GeneratedImage { bytes, mime })
    }

    /// Pull the `errors[0].message` from a Cloudflare JSON error envelope.
    fn error_detail(bytes: &[u8]) -> Option<String> {
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

    // A 1x1 transparent PNG.
    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

    fn decode(content_type: &str, body: &[u8]) -> Result<GeneratedImage, Error> {
        CloudflareAiClient::decode_response(content_type, body)
    }

    #[test]
    fn decodes_base64_from_json_envelope() -> Result<(), Error> {
        let body =
            format!(r#"{{"result":{{"image":"{PNG}"}},"success":true,"errors":[],"messages":[]}}"#);
        let img = decode("application/json", body.as_bytes())?;
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes, STANDARD.decode(PNG)?);
        Ok(())
    }

    #[test]
    fn decodes_data_uri_envelope() -> Result<(), Error> {
        let body = format!(
            r#"{{"result":{{"image":"data:image/png;base64,{PNG}"}},"success":true,"errors":[],"messages":[]}}"#
        );
        let img = decode("application/json", body.as_bytes())?;
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes, STANDARD.decode(PNG)?);
        Ok(())
    }

    #[test]
    fn passes_through_raw_image_bytes() -> Result<(), Error> {
        let png = STANDARD.decode(PNG)?;
        let img = decode("image/png", &png)?;
        assert_eq!(img.mime, "image/png");
        assert_eq!(img.bytes, png);
        Ok(())
    }

    #[test]
    fn rejects_envelope_with_success_false() {
        let body = br#"{"result":{},"success":false,"errors":[{"code":10000,"message":"model not found"}],"messages":[]}"#;
        let err = decode("application/json", body).unwrap_err();
        assert!(err.to_string().contains("model not found"));
    }

    #[test]
    fn rejects_missing_result_image() {
        let body = br#"{"result":{},"success":true,"errors":[],"messages":[]}"#;
        assert!(decode("application/json", body).is_err());
    }

    #[test]
    fn rejects_invalid_base64() {
        let body =
            br#"{"result":{"image":"not base64!!"},"success":true,"errors":[],"messages":[]}"#;
        assert!(decode("application/json", body).is_err());
    }

    #[test]
    fn rejects_neither_image_nor_json() {
        assert!(decode("text/plain", b"not json").is_err());
    }

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
