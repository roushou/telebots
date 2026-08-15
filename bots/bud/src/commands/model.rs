//! `/model` — pick the text-generation model for this chat.

use anyhow::{Result, bail};
use botkit::Reply;
use cloudflare_ai::TextModel;
use telebots_core::Block;

use crate::commands::Ctx;

/// Typed arguments for `/model`.
pub struct ModelArgs {
    pub model: TextModel,
}

impl ModelArgs {
    /// Parse and validate a model alias.
    pub fn parse(raw: &str) -> Result<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            bail!("Usage: /model <name> — see /help for names");
        }
        let model = TextModel::from_alias(raw)
            .ok_or_else(|| anyhow::anyhow!("unknown model \"{raw}\" — see /help"))?;
        Ok(Self { model })
    }

    /// Produce the reply: persist the choice and confirm.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        ctx.storage.set_model(chat_id, self.model).await?;
        let mut b = Block::new();
        b.line(format!(
            "🧠 Model set to {} — {}.",
            self.model,
            self.model.description()
        ));
        Ok(Reply::text(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_a_model() {
        assert!(ModelArgs::parse("").is_err());
        assert!(ModelArgs::parse("   ").is_err());
    }

    #[test]
    fn parse_accepts_aliases() -> anyhow::Result<()> {
        assert_eq!(ModelArgs::parse("r1")?.model, TextModel::DeepseekR132b);
        assert_eq!(ModelArgs::parse("llama-70b")?.model, TextModel::Llama3370b);
        Ok(())
    }

    #[test]
    fn parse_rejects_unknown() {
        assert!(ModelArgs::parse("gpt-4").is_err());
    }
}
