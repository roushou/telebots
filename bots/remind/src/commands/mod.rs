//! Command routing for the Remind bot.
//!
//! The [`Command`] enum is the single source of truth:
//! - `#[derive(CommandSpec)]` generates parsing, the Telegram menu, and
//!   `/help` text,
//! - the [`botkit::Command`] impl produces [`botkit::Reply`] outcomes,
//! - botkit's dispatcher is the single place that sends.

mod cancel;
mod remind;
mod reminders;
mod timezone;

use crate::store::Store;

/// Everything a command needs to produce its reply.
#[derive(Clone)]
pub struct Ctx {
    pub store: Store,
}

#[derive(botkit::CommandSpec, Clone)]
#[command(rename_rule = "snake_case", description = "Remind commands:")]
pub enum Command {
    #[command(description = "Set a reminder: /remind in 15m buy milk")]
    Remind(String),

    #[command(description = "List upcoming reminders")]
    Reminders,

    #[command(description = "Cancel a reminder: /cancel 3")]
    Cancel(String),

    #[command(description = "Set your timezone: /timezone +2")]
    Timezone(String),
}

#[botkit::async_trait]
impl botkit::Command for Command {
    type Ctx = Ctx;

    /// Produce the reply. The match is thin and exhaustive; each variant
    /// parses its arguments and delegates to its command object.
    async fn reply(&self, ctx: &Ctx, req: &botkit::Request) -> anyhow::Result<botkit::Reply> {
        match self {
            Command::Remind(raw) => remind::RemindArgs::parse(raw)?.reply(ctx, req).await,
            Command::Reminders => reminders::Reminders.reply(ctx, req.chat_id).await,
            Command::Cancel(raw) => {
                cancel::CancelArgs::parse(raw)?
                    .reply(ctx, req.chat_id)
                    .await
            }
            Command::Timezone(raw) => {
                timezone::TimezoneArgs::parse(raw)?
                    .reply(ctx, req.chat_id)
                    .await
            }
        }
    }
}
