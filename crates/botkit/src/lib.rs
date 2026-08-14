//! BotKit — the framework bots are built on.
//!
//! Bots depend on botkit (and their data crates); teloxide stays inside
//! botkit. A bot defines a command enum (`#[derive(botkit::CommandSpec)]`),
//! implements [`Command`] for it, and calls [`Bot::run`] from `main`.

mod bot;
pub mod command;
pub mod error;
mod health;
mod metrics;
mod reply;
pub mod request;
pub mod telemetry;

pub use async_trait::async_trait;
pub use bot::{AppConfig, Bot};
pub use botkit_derive::CommandSpec;
pub use command::{Command, CommandSpec};
pub use error::Error;
pub use reply::{BoxFuture, Job, JobCtx, Reply};
pub use request::Request;
pub use telemetry::Telemetry;

/// Re-exports for the `#[derive(CommandSpec)]` macro's generated code.
/// Not public API.
#[doc(hidden)]
pub mod __private {
    pub use teloxide::{
        types::BotCommand,
        utils::command::{BotCommands, CommandDescription, CommandDescriptions, ParseError},
    };
}
