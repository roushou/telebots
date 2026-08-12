//! `/help` — list commands, generated from the enum's descriptions.

use anyhow::Result;
use botkit::Reply;
use telebots_core::Block;
use teloxide::utils::command::BotCommands;

use crate::commands::Command;

/// The `/help` command.
pub struct Help;

impl Help {
    /// Produce the reply: a text block with the command list.
    pub async fn reply(&self) -> Result<Reply> {
        let mut b = Block::new();
        b.line(Command::descriptions().to_string().trim_end().to_string());
        Ok(Reply::Text(b))
    }
}
