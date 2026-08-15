//! `/help` — list commands, generated from the enum's descriptions.

use anyhow::Result;
use botkit::{CommandSpec, Reply};
use telebots_core::Block;

use crate::{commands::Command, render};

/// The `/help` command.
pub struct Help;

impl Help {
    /// Produce the reply: a text block with the command list and models.
    pub async fn reply(&self) -> Result<Reply> {
        let mut b = Block::new();
        b.line(Command::help());
        b.blank();
        b.push_block(render::model_table());
        Ok(Reply::text(b))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn help_lists_commands_and_models() -> anyhow::Result<()> {
        let Reply::Text { block, .. } = Help.reply().await? else {
            anyhow::bail!("expected text reply");
        };
        let text = block.build();
        assert!(text.contains("/reset"));
        assert!(text.contains("/model"));
        assert!(text.contains("llama-3.1-8b"));
        Ok(())
    }
}
