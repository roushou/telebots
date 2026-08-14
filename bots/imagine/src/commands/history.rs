//! `/history` — list recent generations from the persistent store.

use anyhow::Result;
use botkit::Reply;
use telebots_core::Block;

use crate::commands::Ctx;

const HISTORY_LIMIT: usize = 10;

/// The `/history` command.
pub struct History;

impl History {
    /// Produce the reply: a text block listing recent generations.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        let generations = ctx
            .storage
            .recent_generations(chat_id, HISTORY_LIMIT)
            .await?;

        let mut b = Block::new();
        if generations.is_empty() {
            b.line("No images yet — try /imagine <prompt>");
        } else {
            b.line(format!(
                "🎨 Your recent generations ({}):",
                generations.len()
            ));
            for generation in &generations {
                b.line(format!(
                    "{}. {} · {}",
                    generation.id, generation.prompt, generation.model
                ));
            }
        }
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
        })
    }

    #[tokio::test]
    async fn empty_history_has_hint() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let outcome = History.reply(&ctx, 1).await?;
        let Reply::Text { block, .. } = outcome else {
            anyhow::bail!("expected text reply");
        };
        assert_eq!(block.build(), "No images yet — try /imagine <prompt>");
        Ok(())
    }

    #[tokio::test]
    async fn lists_recent_prompts() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        for prompt in ["a cat", "a dog"] {
            ctx.storage
                .add_generation(1, Some(42), prompt, "flux-1-schnell", None)
                .await?;
        }
        let outcome = History.reply(&ctx, 1).await?;
        let Reply::Text { block, .. } = outcome else {
            anyhow::bail!("expected text reply");
        };
        let text = block.build();
        assert!(text.contains("a cat"));
        assert!(text.contains("a dog"));
        Ok(())
    }
}
