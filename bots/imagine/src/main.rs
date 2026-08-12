mod commands;
mod config;
mod generator;

use storage::Storage;
use teloxide::prelude::*;

use crate::{config::Config, generator::Generator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    botkit::Env::load_file(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    botkit::Telemetry::init("imagine");

    let config = Config::from_env()?;
    let bot = Bot::new(config.telegram_bot_token);
    let generator =
        Generator::cloudflare(config.cloudflare_account_id, config.cloudflare_api_token)?;
    let storage = Storage::open(&config.db_path).await?;
    let ctx = commands::Ctx { generator, storage };

    commands::Command::register_menu(&bot).await?;

    let config = botkit::AppConfig {
        service: "imagine",
        version: env!("CARGO_PKG_VERSION"),
        metrics_port: config.metrics_port,
    };
    Ok(botkit::App::new(config, ctx, commands::routes())
        .run(bot)
        .await?)
}
