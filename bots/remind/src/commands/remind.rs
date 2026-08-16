//! `/remind` — set a reminder.

use anyhow::Result;
use botkit::{Reply, Request};
use telebots_core::Time;

use crate::{commands::Ctx, render, when::When};

/// Typed arguments for `/remind`.
pub struct RemindArgs {
    when: String,
    message: String,
}

impl RemindArgs {
    /// Split the raw text into its "when" and "message" parts. The actual
    /// time resolution needs the chat's timezone, so it happens in `reply`.
    pub fn parse(raw: &str) -> Result<Self> {
        let (when, message) = When::split(raw)?;
        Ok(Self { when, message })
    }

    /// Produce the reply: resolve the time in the chat's offset, persist the
    /// reminder, and confirm.
    pub async fn reply(&self, ctx: &Ctx, req: &Request) -> Result<Reply> {
        let offset = ctx.store.utc_offset(req.chat_id).await?;
        let at = When::new(Time::now_secs(), offset).resolve(&self.when)?;
        ctx.store
            .add_reminder(req.chat_id, req.user_id, at, &self.message)
            .await?;
        Ok(Reply::text(render::reminder_confirmed(
            at,
            &self.message,
            offset,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_splits_when_and_message() -> Result<()> {
        let args = RemindArgs::parse("in 15m buy milk")?;
        assert_eq!(args.when, "in 15m");
        assert_eq!(args.message, "buy milk");
        Ok(())
    }

    #[test]
    fn parse_requires_a_message() {
        assert!(RemindArgs::parse("in 15m").is_err());
        assert!(RemindArgs::parse("").is_err());
    }
}
