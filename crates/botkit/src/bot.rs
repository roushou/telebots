//! The bot: owns the poller, the command menu, the dispatcher, and graceful
//! shutdown. Bots build a [`Bot`], hand it their context, and call
//! [`Bot::run`]; teloxide never appears in a bot's code.

use std::time::Duration;

use teloxide::{
    Bot as Api, RequestError,
    dispatching::{Dispatcher, HandlerExt as _, UpdateFilterExt as _, UpdateHandler},
    dptree,
    prelude::Requester,
    requests::ResponseResult,
    types::{Message, Update},
    utils::command::BotCommands,
};

use crate::{
    command::Command,
    error::Error,
    health::Server,
    metrics::Metrics,
    reply::{BoxFuture, Runtime, Supervisor, dispatch},
    request::Request,
};

/// How long shutdown waits for in-flight background jobs.
const DRAIN_GRACE: Duration = Duration::from_secs(15);

/// How often the Telegram `get_me` heartbeat runs.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Application configuration.
#[derive(Debug, Clone, Copy)]
pub struct AppConfig {
    pub service: &'static str,
    pub version: &'static str,
    pub metrics_port: u16,
}

/// A configured bot, ready to run.
pub struct Bot {
    api: Api,
    config: AppConfig,
}

impl Bot {
    /// A bot from its Telegram token and [`AppConfig`].
    pub fn new(token: impl Into<String>, config: AppConfig) -> Self {
        Self {
            api: Api::new(token),
            config,
        }
    }

    /// Run the poller. `C` is the bot's command enum (deriving
    /// [`crate::CommandSpec`] and implementing [`Command`]); `ctx` is the
    /// bot's command context.
    pub async fn run<C>(self, ctx: C::Ctx) -> Result<(), Error>
    where
        C: Command + BotCommands,
    {
        let _span = tracing::info_span!("app", service = self.config.service).entered();

        // A revoked/invalid token must fail fast instead of polling
        // silently into the void.
        let me = self
            .api
            .get_me()
            .await
            .map_err(|e| Error::GetMe(e.to_string()))?;
        tracing::info!(
            "{} started (telegram: @{})",
            self.config.service,
            me.username()
        );

        // Register the Telegram menu from the derived command spec.
        self.api
            .set_my_commands(C::bot_commands())
            .await
            .map_err(|e| Error::Menu(e.to_string()))?;

        let metrics = Metrics::new(self.config.service, self.config.version);
        let port = self.config.metrics_port;
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .map_err(|source| Error::Bind { port, source })?;
        Server::serve(listener, metrics.clone());
        Self::install_panic_hook(metrics.clone());
        Self::spawn_heartbeat(self.api.clone(), metrics.clone());

        let runtime = Runtime {
            supervisor: Supervisor::new(metrics.clone()),
            metrics,
        };
        let mut dispatcher = Dispatcher::builder(self.api, Self::routes::<C>())
            .dependencies(dptree::deps![ctx, runtime.clone()])
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

    /// The handler tree, built from the command enum's derived parser.
    fn routes<C>() -> UpdateHandler<RequestError>
    where
        C: Command + BotCommands,
    {
        dptree::entry().branch(
            Update::filter_message()
                .filter_command::<C>()
                .endpoint(handle::<C>),
        )
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
    fn spawn_heartbeat(bot: Api, metrics: Metrics) {
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

/// The single command endpoint: turn the update into a [`Request`] and route
/// the command's reply through botkit's send point. The boxed future keeps the
/// endpoint `Injectable` when `C` is generic.
fn handle<C: Command>(
    cmd: C,
    bot: Api,
    msg: Message,
    ctx: C::Ctx,
    runtime: Runtime,
) -> BoxFuture<ResponseResult<()>> {
    Box::pin(async move {
        let req = Request::from_message(&msg);
        dispatch(&bot, &msg, &runtime, cmd.reply(&ctx, &req)).await
    })
}
