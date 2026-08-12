//! Command routing.
//!
//! The [`Command`] enum is the single source of truth:
//! - `BotCommands` derives parsing (`filter_command`) and `/help`
//!   (`descriptions`),
//! - [`Command::reply`] produces a [`botkit::Reply`] for each variant,
//! - [`Command::dispatch`] routes it through botkit's single send point.
//!
//! Each command is an object in its own module: typed argument structs
//! (`PriceArgs`, ...) parse the raw string in [`args`], and every command
//! exposes a `reply` method returning a [`Block`]. Commands never touch
//! `send_message`.

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

use coingecko::CoinGeckoClient;
use coinmarketcap::CmcClient;

use self::{
    compare::CompareArgs, convert::ConvertArgs, fear_greed::FearGreed, help::Help, info::InfoArgs,
    market::Market, price::PriceArgs, trending::Trending,
};

/// Everything a command needs to produce its reply.
#[derive(Clone)]
pub struct Ctx {
    pub cmc: CmcClient,
    pub coingecko: CoinGeckoClient,
}

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
    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses its arguments and delegates to the command object.
    async fn reply(&self, ctx: &Ctx) -> anyhow::Result<botkit::Reply> {
        let block = match self {
            Command::Price(raw) => PriceArgs::parse(raw)?.reply(ctx).await,
            Command::Convert(raw) => ConvertArgs::parse(raw)?.reply(ctx).await,
            Command::Market => Market.reply(ctx).await,
            Command::Compare(raw) => CompareArgs::parse(raw)?.reply(ctx).await,
            Command::FearGreed => FearGreed.reply(ctx).await,
            Command::Trending => Trending.reply(ctx).await,
            Command::Info(raw) => InfoArgs::parse(raw)?.reply(ctx).await,
            Command::Help => Help.reply().await,
        }?;
        Ok(botkit::Reply::Text(block))
    }

    /// Route a parsed command through botkit's single send point.
    pub async fn dispatch(
        self,
        bot: Bot,
        msg: Message,
        ctx: Ctx,
        supervisor: botkit::Supervisor,
    ) -> ResponseResult<()> {
        botkit::dispatch(&bot, &msg, &supervisor, self.reply(&ctx)).await
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
