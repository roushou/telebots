//! The runtime: how updates flow from Telegram to a handler's reply and
//! back, plus background-job supervision and observability.

mod branches;
mod dispatch;
mod health;
mod messenger;
mod metrics;
mod supervisor;

pub(crate) use branches::{callback_branch, command_branch, inline_branch, message_branch};
pub(crate) use dispatch::{MAX_MESSAGE_LEN, dispatch};
pub(crate) use health::Server;
pub(crate) use messenger::Messenger;
pub use metrics::UsageReporter;
pub(crate) use metrics::{Health, Metrics};
pub(crate) use supervisor::Supervisor;
