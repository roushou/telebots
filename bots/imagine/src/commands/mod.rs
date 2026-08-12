//! Command routing for the Imagine bot.
//!
//! Commands return explicit [`Outcome`]s — what the bot should do — and
//! [`Command::dispatch`] is the single place that executes them. Commands
//! never touch `send_message`; generation runs in a background task and
//! delivers its photo through the dispatcher.

use storage::{Record, Storage};
use telebots_core::Block;
use teloxide::{
    RequestError,
    dispatching::UpdateHandler,
    prelude::*,
    types::{InputFile, ReplyParameters, Update},
    utils::command::BotCommands,
};

mod help;
mod history;
mod imagine;

use self::{help::Help, history::History, imagine::ImagineArgs};
use crate::generator::Generator;

/// Everything a command needs to produce its outcome.
#[derive(Clone)]
pub struct Ctx {
    pub generator: Generator,
    pub storage: Storage,
}

/// Telegram's message length limit.
const MAX_MESSAGE_LEN: usize = 4096;

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

/// An explicit intent: what the bot should do next.
pub enum Outcome {
    /// Reply with a text block now.
    Text(Block),
    /// Acknowledge with a placeholder, generate in the background, and
    /// deliver the photo (or an error) later.
    Generate(GenerateIntent),
}

/// What to generate.
pub struct GenerateIntent {
    pub prompt: String,
}

impl Command {
    /// Produce the outcome for a parsed command. The match is thin and
    /// exhaustive; each variant parses and delegates to its command object.
    async fn reply(
        &self,
        ctx: &Ctx,
        chat_id: i64,
        user_id: Option<i64>,
    ) -> anyhow::Result<Outcome> {
        match self {
            Command::Imagine(raw) => ImagineArgs::parse(raw)?.reply(ctx, chat_id, user_id).await,
            Command::History => History.reply(ctx, chat_id).await,
            Command::Help => Help.reply().await,
        }
    }

    /// Execute a parsed command: interpret its outcome and send the reply.
    pub async fn dispatch(self, bot: Bot, msg: Message, ctx: Ctx) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        let user_id = msg.from.as_ref().map(|u| u.id.0 as i64);
        match self.reply(&ctx, chat_id, user_id).await {
            Ok(Outcome::Text(block)) => {
                bot.send_message(msg.chat.id, block.truncate(MAX_MESSAGE_LEN).build())
                    .await?;
            }
            Ok(Outcome::Generate(intent)) => {
                let placeholder = bot.send_message(msg.chat.id, "🎨 generating…").await?;
                let bot = bot.clone();
                let ctx = ctx.clone();
                tokio::spawn(async move {
                    Self::deliver_generation(bot, msg, placeholder, ctx, intent).await;
                });
            }
            Err(e) => {
                bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await?;
            }
        }
        Ok(())
    }

    /// The background half of `/imagine`: generate, record history, deliver
    /// the photo (replacing the placeholder) or an error.
    async fn deliver_generation(
        bot: Bot,
        msg: Message,
        placeholder: Message,
        ctx: Ctx,
        intent: GenerateIntent,
    ) {
        match ctx.generator.generate(&intent.prompt).await {
            Ok(image) => {
                // Store a compact JPEG copy, not the full PNG: the record
                // log only lists prompts today, and the DB would otherwise
                // grow by megabytes per generation.
                let payload = match image.compact() {
                    Ok(bytes) => Some(bytes),
                    Err(e) => {
                        tracing::warn!("failed to compact image for storage: {e:#}");
                        None
                    }
                };
                let record = Record {
                    id: None,
                    chat_id: msg.chat.id.0,
                    user_id: msg.from.as_ref().map(|u| u.id.0 as i64),
                    kind: "image".to_string(),
                    text: Some(intent.prompt.clone()),
                    payload,
                    created_at: None,
                };
                if let Err(e) = ctx.storage.append(record).await {
                    tracing::warn!("failed to record history: {e:#}");
                }
                let result = bot
                    .send_photo(msg.chat.id, InputFile::memory(image.bytes))
                    .caption(format!("🎨 {}", intent.prompt))
                    .reply_parameters(ReplyParameters::new(msg.id))
                    .await;
                match result {
                    Ok(_) => {
                        if let Err(e) = bot.delete_message(msg.chat.id, placeholder.id).await {
                            tracing::warn!("failed to delete placeholder: {e}");
                        }
                    }
                    Err(e) => {
                        let _ = bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await;
                    }
                }
            }
            Err(e) => {
                let _ = bot.send_message(msg.chat.id, format!("⚠️ {e:#}")).await;
            }
        }
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
