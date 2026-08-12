//! Command routing for the Imagine bot.
//!
//! Commands return [`botkit::Reply`] outcomes; botkit's dispatcher is the
//! single place that sends. Generation runs in a supervised background job
//! that delivers the photo (or an error) and cleans up the placeholder.

use storage::Storage;
use teloxide::{
    RequestError, dispatching::UpdateHandler, prelude::*, types::Update,
    utils::command::BotCommands,
};

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

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case", description = "Imagine commands:")]
pub enum Command {
    #[command(description = "Generate an image: /imagine <prompt>")]
    Imagine(String),

    #[command(description = "Recent generations")]
    History,

    #[command(description = "Show help")]
    Help,
}

impl Command {
    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses and delegates to its command object.
    async fn reply(
        &self,
        ctx: &Ctx,
        chat_id: i64,
        user_id: Option<i64>,
    ) -> anyhow::Result<botkit::Reply> {
        match self {
            Command::Imagine(raw) => ImagineArgs::parse(raw)?.reply(ctx, chat_id, user_id).await,
            Command::History => History.reply(ctx, chat_id).await,
            Command::Help => Help.reply().await,
        }
    }

    /// Route a parsed command through botkit's single send point.
    pub async fn dispatch(
        self,
        bot: Bot,
        msg: Message,
        ctx: Ctx,
        supervisor: botkit::Supervisor,
    ) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
        botkit::dispatch(&bot, &msg, &supervisor, self.reply(&ctx, chat_id, user_id)).await
    }

    /// Register this command set as the bot's Telegram command menu.
    pub async fn register_menu(bot: &Bot) -> ResponseResult<()> {
        bot.set_my_commands(Command::bot_commands()).await?;
        Ok(())
    }
}

/// The full handler tree. Commands parse to a [`Command`]; dispatch happens
/// in [`Command::dispatch`].
pub fn routes() -> UpdateHandler<RequestError> {
    dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(Command::dispatch),
    )
}
