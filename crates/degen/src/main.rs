mod cmc;
mod coingecko;
mod commands;
mod config;
mod money;

use teloxide::prelude::*;

use crate::{cmc::CmcClient, coingecko::CoinGeckoClient, config::Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "degen=info,teloxide=warn,reqwest=warn".into()),
        )
        .init();

    // Load and validate all env config up front; reports every missing or
    // invalid variable at once instead of panicking on the first.
    let config = Config::from_env()?;

    let bot = Bot::new(config.telegram_bot_token);
    let cmc = CmcClient::new(config.coinmarketcap_api_key);
    let coingecko = CoinGeckoClient::new();

    tracing::info!("degen bot started");

    commands::Command::register_menu(&bot).await?;

    let handler = commands::routes();

    let mut dispatcher = Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![cmc, coingecko])
        .enable_ctrlc_handler()
        .build();

    // On SIGTERM (systemd stop/restart) or Ctrl+C (local dev), shut the
    // dispatcher down gracefully so in-flight updates finish.
    let shutdown_token = dispatcher.shutdown_token();
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        tracing::info!("shutdown signal received");
        let _ = shutdown_token.shutdown();
    });

    // Long polling: no public URL or TLS required.
    dispatcher.dispatch().await;

    Ok(())
}

/// Resolves on SIGTERM (sent by systemd on stop/restart) or Ctrl+C (local dev).
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    }
}
