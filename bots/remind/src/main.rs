mod commands;
mod config;
mod render;
mod scheduler;
mod store;
mod when;

use std::time::Duration;

use crate::{config::Config, scheduler::ReminderSource, store::Store};

/// How often the scheduler checks for due reminders. The first tick fires
/// immediately, so reminders that came due while the bot was offline are
/// delivered on startup.
const SCHEDULE_INTERVAL: Duration = Duration::from_secs(15);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load this bot's own .env (gitignored, per machine) regardless of CWD.
    dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env")).ok();
    botkit::Telemetry::init(env!("CARGO_PKG_NAME"));

    let config = Config::from_env()?;
    let store = Store::open(&config.db_path).await?;
    let ctx = commands::Ctx {
        store: store.clone(),
    };
    let router = botkit::Router::new(ctx)
        .command::<commands::Command>()
        .help::<commands::Command>(Some(render::when_help()))
        .stats();

    botkit::Bot::builder()
        .token(config.telegram_bot_token)
        .service(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .metrics_port(config.metrics_port)
        .scheduler(SCHEDULE_INTERVAL, ReminderSource::new(store))
        .run(router)
        .await?;
    Ok(())
}
