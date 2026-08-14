//! Command routing.
//!
//! The [`Command`] enum is the single source of truth:
//! - `#[derive(CommandSpec)]` generates parsing, the Telegram menu, and
//!   `/help` text,
//! - the [`botkit::Command`] impl produces a [`botkit::Reply`] per variant,
//! - botkit's dispatcher is the single place that sends.
//!
//! Each command is an object in its own module: typed argument structs
//! (`PriceArgs`, ...) parse the raw string in [`args`], and every command
//! exposes a `reply` method returning a [`Block`]. Commands never touch
//! `send_message`.

mod args;
mod compare;
mod convert;
mod fear_greed;
mod help;
mod info;
mod market;
mod price;
mod trending;

pub use args::Symbols;
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

#[derive(botkit::CommandSpec, Clone)]
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

#[botkit::async_trait]
impl botkit::Command for Command {
    type Ctx = Ctx;

    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses its arguments and delegates to the command object. `/price`
    /// returns a reply with a keyboard; the rest are plain text.
    async fn reply(&self, ctx: &Ctx, _req: &botkit::Request) -> anyhow::Result<botkit::Reply> {
        match self {
            Command::Price(raw) => PriceArgs::parse(raw)?.reply(ctx).await,
            Command::Convert(raw) => ConvertArgs::parse(raw)?
                .reply(ctx)
                .await
                .map(botkit::Reply::text),
            Command::Market => Market.reply(ctx).await.map(botkit::Reply::text),
            Command::Compare(raw) => CompareArgs::parse(raw)?
                .reply(ctx)
                .await
                .map(botkit::Reply::text),
            Command::FearGreed => FearGreed.reply(ctx).await.map(botkit::Reply::text),
            Command::Trending => Trending.reply(ctx).await.map(botkit::Reply::text),
            Command::Info(raw) => InfoArgs::parse(raw)?
                .reply(ctx)
                .await
                .map(botkit::Reply::text),
            Command::Help => Help.reply().await.map(botkit::Reply::text),
        }
    }
}
