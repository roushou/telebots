//! `/imagine` — generate an image from a prompt.
//!
//! Returns a [`Reply::Background`] intent; generation runs in a
//! botkit-supervised background task that delivers the photo (or an error)
//! and cleans up the placeholder. Requests are rate-limited per user via
//! the persistent store (nothing in memory).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use botkit::{Job, Reply};

use crate::commands::Ctx;

const MAX_PROMPT_LEN: usize = 400;
const COOLDOWN_SECS: i64 = 30;
/// How long a generation may take before the bot gives up.
const JOB_TIMEOUT: Duration = Duration::from_secs(120);

/// Typed arguments for `/imagine`.
pub struct ImagineArgs {
    pub prompt: String,
}

impl ImagineArgs {
    /// Parse and validate the raw prompt.
    pub fn parse(raw: &str) -> Result<Self> {
        let prompt = raw.trim().to_string();
        if prompt.is_empty() {
            bail!("Usage: /imagine a cat in a spacesuit");
        }
        if prompt.chars().count() > MAX_PROMPT_LEN {
            bail!("prompt too long (max {MAX_PROMPT_LEN} characters)");
        }
        Ok(Self { prompt })
    }

    /// Reject requests that come too close to the previous one. The
    /// cooldown lives in the persistent store, so it survives restarts.
    async fn enforce_cooldown(&self, ctx: &Ctx, chat_id: i64, user_id: Option<i64>) -> Result<()> {
        let Some(user_id) = user_id else {
            return Ok(());
        };
        let key = format!("cooldown:{chat_id}:{user_id}");
        let now = Self::now_secs();
        if let Some(raw) = ctx.storage.kv_get(&key).await?
            && let Ok(text) = String::from_utf8(raw)
            && let Ok(last) = text.parse::<i64>()
            && now - last < COOLDOWN_SECS
        {
            let wait = COOLDOWN_SECS - (now - last);
            bail!("⏳ one image every {COOLDOWN_SECS}s — try again in {wait}s");
        }
        ctx.storage.kv_set(&key, now.to_string().as_bytes()).await?;
        Ok(())
    }

    /// Produce the reply: run generation in the background and deliver the
    /// photo (or an error) when ready.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64, user_id: Option<i64>) -> Result<Reply> {
        self.enforce_cooldown(ctx, chat_id, user_id).await?;

        let ctx = ctx.clone();
        let prompt = self.prompt.clone();
        Ok(Reply::Background {
            placeholder: "🎨 generating…",
            job: Job::new(JOB_TIMEOUT, move |job| {
                Box::pin(async move {
                    let image = ctx.generator.generate(&prompt).await?;
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
                    Ok(Reply::Photo {
                        bytes: image.bytes,
                        caption: Some(format!("🎨 {prompt}")),
                    })
                })
            }),
        })
    }

    fn now_secs() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use storage::Storage;

    use super::*;
    use crate::{commands::Ctx, generator::Generator};

    async fn ctx() -> Ctx {
        Ctx {
            generator: Generator::cloudflare("acct".into(), "tok".into()),
            storage: Storage::open(":memory:").await.unwrap(),
        }
    }

    #[test]
    fn parse_requires_prompt() {
        assert!(ImagineArgs::parse("").is_err());
        assert!(ImagineArgs::parse("   ").is_err());
    }

    #[test]
    fn parse_trims_and_caps_length() {
        let args = ImagineArgs::parse("  a cat  ").unwrap();
        assert_eq!(args.prompt, "a cat");
        let long = "x".repeat(MAX_PROMPT_LEN + 1);
        assert!(ImagineArgs::parse(&long).is_err());
    }

    #[tokio::test]
    async fn cooldown_blocks_repeated_requests() {
        let ctx = ctx().await;
        let args = ImagineArgs::parse("a cat").unwrap();
        assert!(args.reply(&ctx, 1, Some(42)).await.is_ok());
        let outcome = args.reply(&ctx, 1, Some(42)).await;
        let err = outcome.err().expect("second request should be blocked");
        assert!(format!("{err:#}").contains("try again"));
    }

    #[tokio::test]
    async fn cooldown_is_per_user() {
        let ctx = ctx().await;
        let args = ImagineArgs::parse("a cat").unwrap();
        assert!(args.reply(&ctx, 1, Some(42)).await.is_ok());
        assert!(args.reply(&ctx, 1, Some(7)).await.is_ok());
    }
}
