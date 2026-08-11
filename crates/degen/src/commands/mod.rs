//! Command routing.
//!
//! The [`Command`] enum is the single source of truth:
//! - `BotCommands` derives parsing (`filter_command`) and `/help`
//!   (`descriptions`),
//! - [`Command::handle`] is the one exhaustive dispatch point — each variant
//!   delegates to its own module.
//!
//! Per-command logic lives in one module per command (`price.rs`, ...), each
//! exposing a thin `handle` wrapper over a pure, unit-testable `text`.

use teloxide::{
    RequestError, dispatching::UpdateHandler, prelude::*, types::Update,
    utils::command::BotCommands,
};

mod compare;
mod convert;
mod fear_greed;
mod help;
mod info;
mod market;
mod price;
mod trending;
mod util;

use crate::{cmc::CmcClient, coingecko::CoinGeckoClient};

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
    /// Route a parsed command to its handler. The match is thin and
    /// exhaustive; the compiler enforces that every variant is handled.
    pub async fn handle(
        self,
        bot: Bot,
        msg: Message,
        cmc: CmcClient,
        coingecko: CoinGeckoClient,
    ) -> ResponseResult<()> {
        match self {
            Command::Price(args) => price::handle(bot, msg, args, cmc).await,
            Command::Convert(args) => convert::handle(bot, msg, args, cmc).await,
            Command::Market => market::handle(bot, msg, cmc).await,
            Command::Compare(args) => compare::handle(bot, msg, args, cmc).await,
            Command::FearGreed => fear_greed::handle(bot, msg, cmc).await,
            Command::Trending => trending::handle(bot, msg, coingecko).await,
            Command::Info(args) => info::handle(bot, msg, args, cmc).await,
            Command::Help => help::handle(bot, msg).await,
        }
    }

    /// Register this command set as the bot's Telegram command menu.
    pub async fn register_menu(bot: &Bot) -> ResponseResult<()> {
        bot.set_my_commands(Command::bot_commands()).await?;
        Ok(())
    }
}

/// The full handler tree. Commands parse to a [`Command`]; dispatch happens
/// in [`Command::handle`].
pub fn routes() -> UpdateHandler<RequestError> {
    dptree::entry().branch(
        Update::filter_message()
            .filter_command::<Command>()
            .endpoint(Command::handle),
    )
}
