//! `/system` — set (or reset) bud's personality for this chat.

use anyhow::{Result, bail};
use botkit::Reply;
use telebots_core::Block;

use crate::commands::Ctx;

/// Typed arguments for `/system`.
pub struct SystemArgs {
    /// `None` restores the default personality.
    pub prompt: Option<String>,
}

impl SystemArgs {
    /// Parse the raw prompt; `reset`/`default` clear the override.
    pub fn parse(raw: &str) -> Result<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            bail!("Usage: /system <prompt> — or /system reset to restore the default");
        }
        if raw.eq_ignore_ascii_case("reset") || raw.eq_ignore_ascii_case("default") {
            return Ok(Self { prompt: None });
        }
        Ok(Self {
            prompt: Some(raw.to_string()),
        })
    }

    /// Produce the reply: persist (or clear) the prompt and confirm.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        let mut b = Block::new();
        match &self.prompt {
            Some(prompt) => {
                ctx.storage.set_system_prompt(chat_id, prompt).await?;
                b.line("🎭 Personality set.");
            }
            None => {
                ctx.storage.clear_system_prompt(chat_id).await?;
                b.line("🎭 Personality reset to default.");
            }
        }
        Ok(Reply::text(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_a_prompt() {
        assert!(SystemArgs::parse("").is_err());
        assert!(SystemArgs::parse("   ").is_err());
    }

    #[test]
    fn parse_sets_a_prompt() -> anyhow::Result<()> {
        let args = SystemArgs::parse("  be very terse  ")?;
        assert_eq!(args.prompt.as_deref(), Some("be very terse"));
        Ok(())
    }

    #[test]
    fn parse_resets_on_keyword() {
        assert_eq!(SystemArgs::parse("reset").unwrap().prompt, None);
        assert_eq!(SystemArgs::parse("DEFAULT").unwrap().prompt, None);
    }
}
