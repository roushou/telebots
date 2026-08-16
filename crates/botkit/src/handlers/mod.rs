//! The behavior traits a bot implements: [`Command`] for slash commands,
//! [`MessageHandler`] for free-form text, [`InlineHandler`] for inline
//! queries, and [`CallbackHandler`] for button taps.

mod callback;
mod command;
mod inline;
mod message;

pub use callback::CallbackHandler;
pub use command::{Command, CommandSpec, MenuEntry};
pub use inline::{InlineAnswer, InlineHandler, InlineResult};
pub use message::MessageHandler;
