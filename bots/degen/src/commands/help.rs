//! `/help` — list commands, generated from the enum's descriptions.

use anyhow::Result;
use botkit::CommandSpec;
use telebots_core::Block;

use super::Command;

/// The `/help` command.
pub struct Help;

impl Help {
    /// Produce the reply block.
    pub async fn reply(&self) -> Result<Block> {
        let mut b = Block::new();
        b.line(Command::help());
        Ok(b)
    }
}
