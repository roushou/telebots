//! `/cancel` — remove a reminder by id.

use anyhow::{Result, bail};
use botkit::Reply;
use telebots_core::Block;

use crate::commands::Ctx;

/// Typed arguments for `/cancel`.
pub struct CancelArgs {
    id: i64,
}

impl CancelArgs {
    /// Parse and validate a positive reminder id.
    pub fn parse(raw: &str) -> Result<Self> {
        let id: i64 = raw
            .trim()
            .parse()
            .map_err(|_| anyhow::anyhow!("Usage: /cancel <number> — see /reminders"))?;
        if id <= 0 {
            bail!("Usage: /cancel <number> — see /reminders");
        }
        Ok(Self { id })
    }

    /// Produce the reply: delete the reminder (scoped to this chat).
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        let removed = ctx.store.cancel_reminder(chat_id, self.id).await?;
        let mut b = Block::new();
        if removed {
            b.line(format!("✅ Reminder #{} cancelled.", self.id));
        } else {
            b.line(format!(
                "⚠️ No reminder #{} found — see /reminders.",
                self.id
            ));
        }
        Ok(Reply::text(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_a_positive_id() -> Result<()> {
        assert_eq!(CancelArgs::parse("3")?.id, 3);
        assert!(CancelArgs::parse("").is_err());
        assert!(CancelArgs::parse("x").is_err());
        assert!(CancelArgs::parse("0").is_err());
        assert!(CancelArgs::parse("-1").is_err());
        Ok(())
    }
}
