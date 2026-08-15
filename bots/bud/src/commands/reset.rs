//! `/reset` — clear the conversation.

use anyhow::Result;
use botkit::Reply;
use telebots_core::Block;

use crate::commands::Ctx;

/// The `/reset` command.
pub struct Reset;

impl Reset {
    /// Produce the reply: clear the chat's history.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        ctx.storage.clear_chat(chat_id).await?;
        let mut b = Block::new();
        b.line("🧹 Conversation cleared.");
        Ok(Reply::text(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands::Ctx, generator::Generator, store::Store};

    async fn ctx() -> anyhow::Result<Ctx> {
        Ok(Ctx {
            generator: Generator::cloudflare("acct".into(), "tok".into())?,
            storage: Store::open(":memory:").await?,
            default_system_prompt: "be nice".into(),
            max_history: 20,
        })
    }

    #[tokio::test]
    async fn clears_history() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        ctx.storage.add_message(1, Some(42), "user", "hi").await?;
        let _ = Reset.reply(&ctx, 1).await?;
        assert!(ctx.storage.recent_messages(1, 10).await?.is_empty());
        Ok(())
    }
}
