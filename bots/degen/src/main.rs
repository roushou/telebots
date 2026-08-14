mod callback;
mod commands;
mod config;
mod inline;
mod render;

use coingecko::CoinGeckoClient;
use coinmarketcap::CmcClient;

use crate::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init(env!("CARGO_PKG_NAME"));

    let config = Config::from_env()?;
    let cmc = CmcClient::new(config.coinmarketcap_api_key.into_inner())?;
    let coingecko = CoinGeckoClient::new()?;
    let ctx = commands::Ctx { cmc, coingecko };
    let router = botkit::Router::new(ctx)
        .command::<commands::Command>()
        .inline_query(inline::Inline)
        .callback(callback::PriceRefresh)
        .stats();

    botkit::Bot::builder()
        .token(config.telegram_bot_token)
        .service(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .metrics_port(config.metrics_port)
        .run(router)
        .await?;
    Ok(())
}
