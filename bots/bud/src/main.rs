mod commands;
mod config;
mod conversation;
mod cooldown;
mod generator;
mod message;
mod pricing;
mod render;
mod store;

use crate::{config::Config, generator::Generator, store::Store};

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
    let storage = Store::open(&config.db_path).await?;
    let ctx = commands::Ctx {
        generator,
        storage,
        default_system_prompt: config.system_prompt.clone(),
        max_history: config.max_history,
    };
    let router = botkit::Router::new(ctx)
        .command::<commands::Command>()
        .stats()
        .message(message::Chat);

    botkit::Bot::builder()
        .token(config.telegram_bot_token)
        .service(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .metrics_port(config.metrics_port)
        .run(router)
        .await?;
    Ok(())
}
