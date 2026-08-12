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
        let records = ctx.storage.recent(chat_id, "image", HISTORY_LIMIT).await?;

        let mut b = Block::new();
        if records.is_empty() {
            b.line("No images yet — try /imagine <prompt>");
        } else {
            b.line(format!("🎨 Your recent generations ({}):", records.len()));
            for record in &records {
                let prompt = record.text.as_deref().unwrap_or("?");
                let id = record.id.unwrap_or(0);
                b.line(format!("{id}. {prompt}"));
            }
        }
        Ok(Reply::Text(b))
    }
}

#[cfg(test)]
mod tests {
    use storage::{Record, Storage};

    use super::*;
    use crate::{commands::Ctx, generator::Generator};

    async fn ctx() -> anyhow::Result<Ctx> {
        Ok(Ctx {
            generator: Generator::cloudflare("acct".into(), "tok".into())?,
            storage: Storage::open(":memory:").await?,
        })
    }

    #[tokio::test]
    async fn empty_history_has_hint() -> anyhow::Result<()> {
        let ctx = ctx().await?;
        let outcome = History.reply(&ctx, 1).await?;
        let Reply::Text(block) = outcome else {
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
                .append(Record {
                    id: None,
                    chat_id: 1,
                    user_id: Some(42),
                    kind: "image".to_string(),
                    text: Some(prompt.into()),
                    payload: None,
                    created_at: None,
                })
                .await?;
        }
        let outcome = History.reply(&ctx, 1).await?;
        let Reply::Text(block) = outcome else {
            anyhow::bail!("expected text reply");
        };
        let text = block.build();
        assert!(text.contains("a cat"));
        assert!(text.contains("a dog"));
        Ok(())
    }
}
