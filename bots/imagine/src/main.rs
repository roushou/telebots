mod commands;
mod config;
mod generator;

use storage::Storage;

use crate::{config::Config, generator::Generator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init("imagine");

    let config = Config::from_env()?;
    let generator =
        Generator::cloudflare(config.cloudflare_account_id, config.cloudflare_api_token)?;
    let storage = Storage::open(&config.db_path).await?;
    let ctx = commands::Ctx { generator, storage };

    let bot = botkit::Bot::new(
        config.telegram_bot_token,
        botkit::AppConfig {
            service: "imagine",
            version: env!("CARGO_PKG_VERSION"),
            metrics_port: config.metrics_port,
        },
    );
    bot.run::<commands::Command>(ctx).await?;
    Ok(())
}
