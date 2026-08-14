//! `/imagine` — generate an image from a prompt.
//!
//! Returns a [`Reply::Background`] intent; generation runs in a
//! botkit-supervised background task that delivers the photo (or an error)
//! and cleans up the placeholder. Rate limiting is a router guard (see
//! `cooldown.rs`).

use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use botkit::{Job, Reply};
use cloudflare_ai::Model;

use crate::commands::Ctx;

const MAX_PROMPT_LEN: usize = 400;
/// How long a generation may take before the bot gives up.
const JOB_TIMEOUT: Duration = Duration::from_secs(300);

/// Typed arguments for `/imagine`.
pub struct ImagineArgs {
    pub model: Model,
    pub prompt: String,
}

impl ImagineArgs {
    /// Parse and validate the raw prompt, with an optional leading model.
    pub fn parse(raw: &str) -> Result<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            bail!("Usage: /imagine <prompt> — or /imagine <model> <prompt>");
        }

        let (model, prompt) = match raw.split_once(|c: char| c.is_whitespace()) {
            Some((first, rest)) => match Model::from_alias(first) {
                Some(model) => (model, rest.trim().to_string()),
                None => (Model::default(), raw.to_string()),
            },
            // A lone token is a prompt, unless it is a model alias with no
            // prompt — that is a usage error.
            None => match Model::from_alias(raw) {
                Some(model) => bail!("Usage: /imagine {model} <prompt>"),
                None => (Model::default(), raw.to_string()),
            },
        };

        if prompt.chars().count() > MAX_PROMPT_LEN {
            bail!("prompt too long (max {MAX_PROMPT_LEN} characters)");
        }
        Ok(Self { model, prompt })
    }

    /// The placeholder shown while generation runs, tuned to the model.
    fn placeholder(&self) -> &'static str {
        match self.model {
            Model::Flux2Dev => "🎨 generating with flux-2-dev… (can take a few minutes)",
            Model::Flux2Klein4b | Model::Flux2Klein9b => {
                "🎨 generating with flux-2-klein… (can take a few minutes)"
            }
            Model::Flux1Schnell => "🎨 generating with flux-1-schnell…",
            Model::SdXlLightning => "🎨 generating with sd-xl-lightning…",
            Model::Dreamshaper8Lcm => "🎨 generating with dreamshaper-8-lcm…",
            Model::SdXlBase1 => "🎨 generating with sd-xl-base…",
            _ => "🎨 generating…",
        }
    }

    /// Produce the reply: run generation in the background and deliver the
    /// photo (or an error) when ready.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Reply> {
        let ctx = ctx.clone();
        let prompt = self.prompt.clone();
        let model = self.model;
        Ok(Reply::Background {
            placeholder: self.placeholder().to_string(),
            job: Job::new(JOB_TIMEOUT, move |job| {
                Box::pin(async move {
                    let started = Instant::now();
                    tracing::info!(
                        model = %model,
                        prompt_len = prompt.chars().count(),
                        "generating image"
                    );
                    let image = ctx.generator.generate(model, &prompt).await?;
                    tracing::info!(
                        model = %model,
                        elapsed = ?started.elapsed(),
                        "image generated"
                    );
                    // Store a compact JPEG copy, not the full PNG — the DB
                    // would otherwise grow by megabytes per generation.
                    let payload = match image.compact() {
                        Ok(bytes) => Some(bytes),
                        Err(e) => {
                            tracing::warn!("failed to compact image for storage: {e:#}");
                            None
                        }
                    };
                    let record = storage::Record {
                        id: None,
                        chat_id: job.chat_id,
                        user_id: job.user_id,
                        kind: "image".to_string(),
                        text: Some(prompt.clone()),
                        payload,
                        created_at: None,
                    };
                    if let Err(e) = ctx.storage.append(record).await {
                        tracing::warn!("failed to record history: {e:#}");
                    }
                    Ok(Reply::photo(
                        image.bytes,
                        Some(format!("🎨 {prompt} · {model}")),
                    ))
                })
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_prompt() {
        assert!(ImagineArgs::parse("").is_err());
        assert!(ImagineArgs::parse("   ").is_err());
    }

    #[test]
    fn parse_trims_and_caps_length() -> anyhow::Result<()> {
        let args = ImagineArgs::parse("  a cat  ")?;
        assert_eq!(args.model, Model::default());
        assert_eq!(args.prompt, "a cat");
        let long = "x".repeat(MAX_PROMPT_LEN + 1);
        assert!(ImagineArgs::parse(&long).is_err());
        Ok(())
    }

    #[test]
    fn parse_selects_model_from_leading_token() -> anyhow::Result<()> {
        let args = ImagineArgs::parse("schnell a cat")?;
        assert_eq!(args.model, Model::Flux1Schnell);
        assert_eq!(args.prompt, "a cat");
        Ok(())
    }

    #[test]
    fn parse_treats_unknown_first_word_as_prompt() -> anyhow::Result<()> {
        let args = ImagineArgs::parse("lightning strikes a tower")?;
        assert_eq!(args.model, Model::default());
        assert_eq!(args.prompt, "lightning strikes a tower");
        Ok(())
    }

    #[test]
    fn parse_requires_prompt_after_model() {
        assert!(ImagineArgs::parse("flux-2-dev").is_err());
        assert!(ImagineArgs::parse("flux-2-dev   ").is_err());
    }
}
