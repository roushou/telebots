//! Typed botkit errors.

use thiserror::Error;

/// Errors surfaced while starting or running the bot.
///
/// The public surface is deliberately free of teloxide types; transport
/// errors are folded into the string-carrying variants.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// The Telegram token was rejected (revoked or invalid).
    #[error("getMe failed — check the bot token ({0})")]
    GetMe(String),

    /// Registering the command menu with Telegram failed.
    #[error("failed to register the command menu: {0}")]
    Menu(String),

    /// The metrics port could not be bound.
    #[error("failed to bind metrics port {port}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },
}
