//! Command routing for the Bud bot.
//!
//! The [`Command`] enum is the single source of truth:
//! - `#[derive(CommandSpec)]` generates parsing, the Telegram menu, and
//!   `/help` text,
//! - the [`botkit::Command`] impl produces [`botkit::Reply`] outcomes,
//! - botkit's dispatcher is the single place that sends.
//!
//! Free-form chat is *not* a command — it is handled by the
//! [`crate::message::Chat`] message handler.

mod history;
mod model;
mod reset;
mod system;

use self::{history::History, model::ModelArgs, reset::Reset, system::SystemArgs};
use crate::{generator::Generator, store::Store};

/// Everything a command needs to produce its reply.
#[derive(Clone)]
pub struct Ctx {
    pub generator: Generator,
    pub storage: Store,
    pub default_system_prompt: String,
    pub max_history: usize,
}

#[derive(botkit::CommandSpec, Clone)]
#[command(rename_rule = "snake_case", description = "Bud commands:")]
pub enum Command {
    #[command(description = "Clear the conversation")]
    Reset,

    #[command(description = "Show your recent conversation")]
    History,

    #[command(description = "Pick the AI model: /model <name>")]
    Model(String),

    #[command(description = "Set bud's personality: /system <prompt>")]
    System(String),
}

#[botkit::async_trait]
impl botkit::Command for Command {
    type Ctx = Ctx;

    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses its arguments and delegates to its command object.
    async fn reply(&self, ctx: &Ctx, req: &botkit::Request) -> anyhow::Result<botkit::Reply> {
        match self {
            Command::Reset => Reset.reply(ctx, req.chat_id).await,
            Command::History => History.reply(ctx, req.chat_id).await,
            Command::Model(raw) => ModelArgs::parse(raw)?.reply(ctx, req.chat_id).await,
            Command::System(raw) => SystemArgs::parse(raw)?.reply(ctx, req.chat_id).await,
        }
    }
}
