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
    let ctx = commands::Ctx {
        generator: Generator::cloudflare(config.cloudflare_account_id, config.cloudflare_api_token),
        storage: Storage::open(&config.db_path).await?,
    };

    commands::Command::register_menu(&bot).await?;

    botkit::App::new(
        "imagine",
        env!("CARGO_PKG_VERSION"),
        ctx,
        commands::routes(),
    )
    .run(bot)
    .await
}
