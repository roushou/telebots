//! The bot: owns the poller, the command menu, the dispatcher, and graceful
//! shutdown. Bots build a [`Bot`], hand it their context, and call
//! [`Bot::run`]; teloxide never appears in a bot's code.

use std::{sync::Arc, time::Duration};

use teloxide::{
    Bot as Api, RequestError,
    dispatching::{Dispatcher, UpdateFilterExt as _, UpdateHandler},
    dptree,
    error_handlers::ErrorHandler,
    prelude::Requester,
    requests::ResponseResult,
    types::{BotCommand, Me, Message, Update},
};

use crate::{
    command::Command,
    error::Error,
    health::Server,
    metrics::Metrics,
    reply::{BoxFuture, Supervisor, dispatch},
    request::Request,
};

/// How long shutdown waits for in-flight background jobs.
const DRAIN_GRACE: Duration = Duration::from_secs(15);

/// How often the Telegram `get_me` heartbeat runs.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// A configured bot, ready to run.
pub struct Bot {
    api: Api,
    service: &'static str,
    version: &'static str,
    metrics_port: u16,
    metrics_addr: &'static str,
}

impl Bot {
    /// Start building a bot.
    pub fn builder() -> BotBuilder {
        BotBuilder {
            token: None,
            service: "",
            version: "",
            metrics_port: 0,
            metrics_addr: "0.0.0.0",
        }
    }

    /// Run the poller. `C` is the bot's command enum (deriving
    /// [`crate::CommandSpec`] and implementing [`Command`]); `ctx` is the
    /// bot's command context.
    pub async fn run<C>(self, ctx: C::Ctx) -> Result<(), Error>
    where
        C: Command,
    {
        let _span = tracing::info_span!("app", service = self.service).entered();

        // A revoked/invalid token must fail fast instead of polling
        // silently into the void.
        let me = self
            .api
            .get_me()
            .await
            .map_err(|e| Error::GetMe(e.to_string()))?;
        tracing::info!("{} started (telegram: @{})", self.service, me.username());

        // Register the Telegram menu from the derived command spec. A
        // failure only costs the `/` autocomplete menu, so warn instead of
        // aborting startup.
        if let Err(e) = self
            .api
            .set_my_commands(
                C::menu()
                    .into_iter()
                    .map(|entry| BotCommand::new(entry.command, entry.description)),
            )
            .await
        {
            tracing::warn!("failed to register the command menu: {e}");
        }

        let metrics = Metrics::new(self.service, self.version);
        let port = self.metrics_port;
        let listener = tokio::net::TcpListener::bind((self.metrics_addr, port))
            .await
            .map_err(|source| Error::Bind { port, source })?;
        Server::serve(listener, metrics.clone());
        Self::install_panic_hook(metrics.clone());
        Self::spawn_heartbeat(self.api.clone(), metrics.clone());

        let supervisor = Supervisor::new(metrics.clone());
        let mut dispatcher = Dispatcher::builder(self.api, Self::routes::<C>())
            .dependencies(dptree::deps![ctx, supervisor.clone()])
            .error_handler(Self::dispatch_error_handler(metrics))
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
        supervisor.drain(DRAIN_GRACE).await;
        Ok(())
    }

    /// The handler tree, built from the command enum's derived parser.
    fn routes<C>() -> UpdateHandler<RequestError>
    where
        C: Command,
    {
        dptree::entry().branch(
            Update::filter_message()
                .filter_map(|msg: Message, me: Me| {
                    let bot_name = me.user.username.as_deref()?;
                    let text = msg.text().or_else(|| msg.caption())?;
                    C::parse(text, bot_name)
                })
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

    /// The dispatcher's error handler: count delivery failures and log them.
    fn dispatch_error_handler(
        metrics: Metrics,
    ) -> Arc<dyn ErrorHandler<RequestError> + Send + Sync> {
        Arc::new(move |err: RequestError| {
            metrics.note_dispatch_error();
            async move {
                tracing::warn!("update handling failed: {err}");
            }
        })
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

/// Builder for [`Bot`].
#[derive(Debug, Clone)]
pub struct BotBuilder {
    token: Option<String>,
    service: &'static str,
    version: &'static str,
    metrics_port: u16,
    metrics_addr: &'static str,
}

impl BotBuilder {
    /// The Telegram bot token.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// The service name, used in logs and metrics.
    pub fn service(mut self, service: &'static str) -> Self {
        self.service = service;
        self
    }

    /// The service version, reported in metrics.
    pub fn version(mut self, version: &'static str) -> Self {
        self.version = version;
        self
    }

    /// The port the metrics server binds.
    pub fn metrics_port(mut self, port: u16) -> Self {
        self.metrics_port = port;
        self
    }

    /// The address the metrics server binds (default `0.0.0.0`).
    pub fn metrics_addr(mut self, addr: &'static str) -> Self {
        self.metrics_addr = addr;
        self
    }

    /// Assemble the bot, failing if no token was provided.
    pub fn build(self) -> Result<Bot, Error> {
        let token = self.token.ok_or(Error::MissingToken)?;
        Ok(Bot {
            api: Api::new(token),
            service: self.service,
            version: self.version,
            metrics_port: self.metrics_port,
            metrics_addr: self.metrics_addr,
        })
    }

    /// Build and run the poller.
    pub async fn run<C>(self, ctx: C::Ctx) -> Result<(), Error>
    where
        C: Command,
    {
        self.build()?.run::<C>(ctx).await
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
    supervisor: Supervisor,
) -> BoxFuture<ResponseResult<()>> {
    Box::pin(async move {
        let req = Request::from_message(&msg);
        dispatch(&bot, &msg, &supervisor, cmd.reply(&ctx, &req)).await
    })
}
