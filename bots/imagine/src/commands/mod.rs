//! Command routing for the Imagine bot.
//!
//! The [`Command`] enum is the single source of truth:
//! - `#[derive(CommandSpec)]` generates parsing, the Telegram menu, and
//!   `/help` text,
//! - the [`botkit::Command`] impl produces [`botkit::Reply`] outcomes,
//! - botkit's dispatcher is the single place that sends.
//!
//! Generation runs in a botkit-supervised background job that delivers the
//! photo (or an error) and cleans up the placeholder.

use storage::Storage;

mod help;
mod history;
mod imagine;

use self::{help::Help, history::History, imagine::ImagineArgs};
use crate::generator::Generator;

/// Everything a command needs to produce its reply.
#[derive(Clone)]
pub struct Ctx {
    pub generator: Generator,
    pub storage: Storage,
}

#[derive(botkit::CommandSpec, Clone)]
#[command(rename_rule = "snake_case", description = "Imagine commands:")]
pub enum Command {
    #[command(description = "Generate an image: /imagine [model] <prompt>")]
    Imagine(String),

    #[command(description = "Recent generations")]
    History,

    #[command(description = "Show help")]
    Help,
}

#[botkit::async_trait]
impl botkit::Command for Command {
    type Ctx = Ctx;

    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses and delegates to its command object.
    async fn reply(&self, ctx: &Ctx, req: &botkit::Request) -> anyhow::Result<botkit::Reply> {
        match self {
            Command::Imagine(raw) => {
                ImagineArgs::parse(raw)?
                    .reply(ctx, req.chat_id, req.user_id)
                    .await
            }
            Command::History => History.reply(ctx, req.chat_id).await,
            Command::Help => Help.reply().await,
        }
    }
}
