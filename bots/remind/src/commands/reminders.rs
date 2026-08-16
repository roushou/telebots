//! `/reminders` — list a chat's upcoming reminders.

use anyhow::Result;
use botkit::Reply;

use crate::{commands::Ctx, render};

/// The `/reminders` command.
pub struct Reminders;

impl Reminders {
    /// Produce the reply: the chat's pending reminders, soonest first.
    pub async fn reply(&self, ctx: &Ctx, chat_id: i64) -> Result<Reply> {
        let offset = ctx.store.utc_offset(chat_id).await?;
        let reminders = ctx.store.list_reminders(chat_id).await?;
        Ok(Reply::text(render::list_reminders(&reminders, offset)))
    }
}
