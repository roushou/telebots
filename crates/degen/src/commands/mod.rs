//! Command routing.
//!
//! The [`Command`] enum is the single source of truth:
//! - `BotCommands` derives parsing (`filter_command`) and `/help`
//!   (`descriptions`),
//! - [`Command::reply`] produces a [`Block`] for each variant,
//! - [`Command::dispatch`] is the single place that sends replies — it
//!   renders the block (with a Telegram message-length cap) or the error.
//!
//! Each command is an object in its own module: typed argument structs
//! (`PriceArgs`, ...) parse the raw string in [`args`], and every command
//! exposes a `reply` method returning a [`Block`]. Commands never touch
//! `send_message`.

use telebots_core::Block;
use teloxide::{
    RequestError, dispatching::UpdateHandler, prelude::*, types::Update,
    utils::command::BotCommands,
};

mod args;
mod compare;
mod convert;
mod fear_greed;
mod help;
mod info;
mod market;
mod price;
mod trending;

use self::{
    compare::CompareArgs, convert::ConvertArgs, fear_greed::FearGreed, help::Help, info::InfoArgs,
    market::Market, price::PriceArgs, trending::Trending,
};
use crate::{cmc::CmcClient, coingecko::CoinGeckoClient};

/// Everything a command needs to produce its reply.
#[derive(Clone)]
pub struct Ctx {
    pub cmc: CmcClient,
    pub coingecko: CoinGeckoClient,
}

/// Telegram's message length limit.
const MAX_MESSAGE_LEN: usize = 4096;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case", description = "Degen commands:")]
pub enum Command {
    #[command(description = "Get prices: /price btc eth")]
    Price(String),

    #[command(description = "Convert: /convert 100 btc usd")]
    Convert(String),

    #[command(description = "Market overview")]
    Market,

    #[command(description = "Compare: /compare btc eth")]
    Compare(String),

    #[command(description = "Fear & Greed index")]
    FearGreed,

    #[command(description = "Trending coins")]
    Trending,

    #[command(description = "Project info: /info btc")]
    Info(String),

    #[command(description = "Show help")]
    Help,
}

impl Command {
    /// Produce the reply block. The match is thin and exhaustive; each
    /// variant parses its arguments and delegates to the command object.
    async fn reply(&self, ctx: &Ctx) -> anyhow::Result<Block> {
        match self {
            Command::Price(raw) => PriceArgs::parse(raw)?.reply(ctx).await,
            Command::Convert(raw) => ConvertArgs::parse(raw)?.reply(ctx).await,
            Command::Market => Market.reply(ctx).await,
            Command::Compare(raw) => CompareArgs::parse(raw)?.reply(ctx).await,
            Command::FearGreed => FearGreed.reply(ctx).await,
            Command::Trending => Trending.reply(ctx).await,
            Command::Info(raw) => InfoArgs::parse(raw)?.reply(ctx).await,
            Command::Help => Help.reply().await,
        }
    }

    /// Route a parsed command: produce its reply and send it. The only place
    /// in the command path that touches `send_message`; errors render as a
    /// uniform `⚠️` message, and replies are capped at Telegram's limit.
    pub async fn dispatch(self, bot: Bot, msg: Message, ctx: Ctx) -> ResponseResult<()> {
        let text = match self.reply(&ctx).await {
            Ok(block) => block.truncate(MAX_MESSAGE_LEN).build(),
            Err(e) => format!("⚠️ {e:#}"),
        };
        bot.send_message(msg.chat.id, text).await?;
        Ok(())
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
