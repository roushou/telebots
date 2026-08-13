//! `/help` — list commands, generated from the enum's descriptions.

use anyhow::Result;
use botkit::Reply;
use cloudflare_ai::Model;
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
        b.blank();
        b.line("Models — prefix /imagine with one (default flux-1-schnell):");
        for model in Model::ALL {
            b.row([model.aliases().join(", "), model.description().to_string()]);
        }
        Ok(Reply::Text(b))
    }
}
