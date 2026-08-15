//! `/history` — list recent conversation messages from the persistent store.

use anyhow::Result;
use botkit::Reply;

use crate::{commands::Ctx, render};

const HISTORY_LIMIT: usize = 20;

/// The `/history` command.
pub struct History;

impl History {
    /// Produce the reply: a text block listing recent messages.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        let messages = ctx.storage.recent_messages(chat_id, HISTORY_LIMIT).await?;
        Ok(Reply::text(render::history_block(&messages)))
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
    async fn empty_history_has_hint() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let Reply::Text { block, .. } = History.reply(&ctx, 1).await? else {
            anyhow::bail!("expected text reply");
        };
        assert!(block.build().contains("No messages yet"));
        Ok(())
    }

    #[tokio::test]
    async fn lists_recent_messages() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        ctx.storage.add_message(1, Some(42), "user", "hi").await?;
        ctx.storage
            .add_message(1, Some(42), "assistant", "hey")
            .await?;
        let Reply::Text { block, .. } = History.reply(&ctx, 1).await? else {
            anyhow::bail!("expected text reply");
        };
        let text = block.build();
        assert!(text.contains("you: hi"));
        assert!(text.contains("bud: hey"));
        Ok(())
    }
}
