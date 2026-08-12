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
    let cmc = CmcClient::new(config.coinmarketcap_api_key)?;
    let coingecko = CoinGeckoClient::new()?;
    let ctx = commands::Ctx { cmc, coingecko };

    commands::Command::register_menu(&bot).await?;

    let config = botkit::AppConfig {
        service: "degen",
        version: env!("CARGO_PKG_VERSION"),
        metrics_port: config.metrics_port,
    };
    Ok(botkit::App::new(config, ctx, commands::routes())
        .run(bot)
        .await?)
}
