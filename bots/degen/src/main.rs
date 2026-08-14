mod commands;
mod config;
mod render;

use coingecko::CoinGeckoClient;
use coinmarketcap::CmcClient;

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init("degen");

    let config = Config::from_env()?;
    let cmc = CmcClient::new(config.coinmarketcap_api_key)?;
    let coingecko = CoinGeckoClient::new()?;
    let ctx = commands::Ctx { cmc, coingecko };

    let bot = botkit::Bot::new(
        config.telegram_bot_token,
        botkit::AppConfig {
            service: "degen",
            version: env!("CARGO_PKG_VERSION"),
            metrics_port: config.metrics_port,
        },
    );
    bot.run::<commands::Command>(ctx).await?;
    Ok(())
}
