//! The dispatcher runner: builds the poller, wires graceful shutdown.

use anyhow::{Context, Result};
use teloxide::{RequestError, dispatching::UpdateHandler, prelude::*};

/// The shell every bot is made of. The bot supplies its own `Ctx`, handler
/// tree, and menu registration; [`App::run`] owns the dispatcher, the
/// startup self-check, and graceful shutdown.
pub struct App<C> {
    service: &'static str,
    ctx: C,
    routes: UpdateHandler<RequestError>,
}

impl<C: Clone + Send + Sync + 'static> App<C> {
    /// A new shell for `service`, with the bot's context and handler tree.
    pub fn new(service: &'static str, ctx: C, routes: UpdateHandler<RequestError>) -> Self {
        Self {
            service,
            ctx,
            routes,
        }
    }

    /// Run the poller. `bot` is the configured bot; the caller registers
    /// the command menu on it first.
    pub async fn run(self, bot: Bot) -> Result<()> {
        let _span = tracing::info_span!("app", service = self.service).entered();

        // A revoked/invalid token must fail fast instead of polling
        // silently into the void.
        let me = bot
            .get_me()
            .await
            .context("getMe failed — check the bot token")?;
        tracing::info!("{} started (telegram: @{})", self.service, me.username());

        let mut dispatcher = Dispatcher::builder(bot, self.routes)
            .dependencies(dptree::deps![self.ctx])
            .enable_ctrlc_handler()
            .build();

        let shutdown_token = dispatcher.shutdown_token();
        tokio::spawn(async move {
            Self::wait_for_shutdown_signal().await;
            tracing::info!("shutdown signal received");
            let _ = shutdown_token.shutdown();
        });

        dispatcher.dispatch().await;
        Ok(())
    }

    /// Resolves on SIGTERM (container stop/restart) or Ctrl+C (local dev).
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
}
