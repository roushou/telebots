//! `/help` — list commands, generated from the enum's descriptions.

use anyhow::Result;
use telebots_core::Block;
use teloxide::utils::command::BotCommands;

use crate::commands::{Command, Outcome};

/// The `/help` command.
pub struct Help;

impl Help {
    /// Produce the outcome: a text block with the command list.
    pub async fn reply(&self) -> Result<Outcome> {
        let mut b = Block::new();
        b.line(Command::descriptions().to_string().trim_end().to_string());
        Ok(Outcome::Text(b))
    }
}
