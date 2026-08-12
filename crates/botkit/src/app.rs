//! The dispatcher runner: builds the poller, wires graceful shutdown, the
//! Telegram heartbeat, and the metrics server.

use std::time::Duration;

use teloxide::{RequestError, dispatching::UpdateHandler, prelude::*};
use thiserror::Error;

use crate::{
    health::Server,
    metrics::Metrics,
    reply::{Runtime, Supervisor},
};

/// Errors surfaced while starting the bot shell.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AppError {
    /// The Telegram token was rejected (revoked or invalid).
    #[error("getMe failed — check the bot token")]
    GetMe(#[source] teloxide::RequestError),

    /// The metrics port could not be bound.
    #[error("failed to bind metrics port {port}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
}

/// How long shutdown waits for in-flight background jobs.
const DRAIN_GRACE: Duration = Duration::from_secs(15);

/// How often the Telegram `get_me` heartbeat runs.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Application-provided identity and runtime knobs for one bot shell. The
/// framework consumes this; it never derives it from a service name.
#[derive(Debug, Clone, Copy)]
pub struct AppConfig {
    pub service: &'static str,
    pub version: &'static str,
    pub metrics_port: u16,
}

/// The shell every bot is made of. The bot supplies its own `Ctx`, handler
/// tree, and menu registration; [`App::run`] owns the dispatcher, the
/// startup self-check, the heartbeat, and graceful shutdown.
pub struct App<C> {
    config: AppConfig,
    ctx: C,
    routes: UpdateHandler<RequestError>,
}

impl<C: Clone + Send + Sync + 'static> App<C> {
    /// A new shell from the application's [`AppConfig`], with the bot's
    /// context and handler tree.
    pub fn new(config: AppConfig, ctx: C, routes: UpdateHandler<RequestError>) -> Self {
        Self {
            config,
            ctx,
            routes,
        }
    }

    /// Run the poller. `bot` is the configured bot; the caller registers
    /// the command menu on it first.
    pub async fn run(self, bot: Bot) -> Result<(), AppError> {
        let _span = tracing::info_span!("app", service = self.config.service).entered();

        // A revoked/invalid token must fail fast instead of polling
        // silently into the void.
        let me = bot.get_me().await.map_err(AppError::GetMe)?;
        tracing::info!(
            "{} started (telegram: @{})",
            self.config.service,
            me.username()
        );

        let metrics = Metrics::new(self.config.service, self.config.version);
        let port = self.config.metrics_port;
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(|source| AppError::Bind { port, source })?;
        Server::serve(listener, metrics.clone());
        Self::install_panic_hook(metrics.clone());
        Self::spawn_heartbeat(bot.clone(), metrics.clone());

        let runtime = Runtime {
            supervisor: Supervisor::new(metrics.clone()),
            metrics,
        };
        let mut dispatcher = Dispatcher::builder(bot, self.routes)
            .dependencies(dptree::deps![self.ctx, runtime.clone()])
            .enable_ctrlc_handler()
            .build();

        let shutdown_token = dispatcher.shutdown_token();
        tokio::spawn(async move {
            if let Err(e) = Self::wait_for_shutdown_signal().await {
                tracing::error!("failed to install shutdown signal handler: {e}");
                return;
            }
            tracing::info!("shutdown signal received");
            let _ = shutdown_token.shutdown();
        });

        dispatcher.dispatch().await;

        // Drain in-flight background jobs before exiting, so a generation
        // isn't cancelled mid-write.
        runtime.supervisor.drain(DRAIN_GRACE).await;
        Ok(())
    }

    /// Log panics with location and count them in the metrics.
    fn install_panic_hook(metrics: Metrics) {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            default_hook(info);
            metrics.note_panic();
            let location = info.location().map(|l| l.to_string());
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("(no message)");
            tracing::error!(target: "panic", location, "panicked: {message}");
        }));
    }

    /// Probe Telegram reachability every [`HEARTBEAT_INTERVAL`].
    fn spawn_heartbeat(bot: Bot, metrics: Metrics) {
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(HEARTBEAT_INTERVAL);
            loop {
                tick.tick().await;
                match bot.get_me().await {
                    Ok(_) => metrics.heartbeat_ok(),
                    Err(e) => {
                        tracing::warn!("telegram heartbeat failed: {e}");
                        metrics.heartbeat_failed();
                    }
                }
            }
        });
    }

    /// Resolves on SIGTERM (container stop/restart) or Ctrl+C (local dev).
    async fn wait_for_shutdown_signal() -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigterm = signal(SignalKind::terminate())?;
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            tokio::signal::ctrl_c().await
        }
    }
}
