//! `/imagine` — generate an image from a prompt.
//!
//! Returns a [`Generate`](crate::commands::Outcome::Generate) intent; the
//! actual generation and photo delivery run in a background task spawned by
//! the dispatcher. Requests are rate-limited per user via the persistent
//! store (nothing in memory).

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};

use crate::commands::{Ctx, GenerateIntent, Outcome};

const MAX_PROMPT_LEN: usize = 400;
const COOLDOWN_SECS: i64 = 30;

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

    /// Produce the outcome: a generate intent (or a cooldown error).
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64, user_id: Option<i64>) -> Result<Outcome> {
        self.enforce_cooldown(ctx, chat_id, user_id).await?;
        Ok(Outcome::Generate(GenerateIntent {
            prompt: self.prompt.clone(),
        }))
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
