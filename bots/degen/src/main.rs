mod commands;
mod config;
mod render;

use coingecko::CoinGeckoClient;
use coinmarketcap::CmcClient;
use teloxide::prelude::*;

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    botkit::Env::load_file(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    botkit::Telemetry::init("degen");

    let config = Config::from_env()?;
    let bot = Bot::new(config.telegram_bot_token);
    let ctx = commands::Ctx {
        cmc: CmcClient::new(config.coinmarketcap_api_key),
        coingecko: CoinGeckoClient::new(),
    };

    commands::Command::register_menu(&bot).await?;

    botkit::App::new("degen", env!("CARGO_PKG_VERSION"), ctx, commands::routes())
        .run(bot)
        .await
}
