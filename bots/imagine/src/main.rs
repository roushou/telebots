mod commands;
mod config;
mod cooldown;
mod generator;

use storage::Storage;

use crate::{config::Config, generator::Generator};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init(env!("CARGO_PKG_NAME"));

    let config = Config::from_env()?;
    let generator = Generator::cloudflare(
        config.cloudflare_account_id.into_inner(),
        config.cloudflare_api_token.into_inner(),
    )?;
    let storage = Storage::open(&config.db_path).await?;
    let ctx = commands::Ctx { generator, storage };
    let router = botkit::Router::new(ctx)
        .guarded_command::<commands::Command, cooldown::Cooldown>(cooldown::Cooldown)
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
