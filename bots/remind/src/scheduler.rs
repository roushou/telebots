//! The schedule source: feeds due reminders into botkit's scheduler loop.

use anyhow::Result;
use botkit::{ScheduleSource, ScheduledMessage};

use crate::{render, store::Store};

/// Reads due reminders from the store and renders them for delivery.
pub struct ReminderSource {
    store: Store,
}

impl ReminderSource {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

#[botkit::async_trait]
impl ScheduleSource for ReminderSource {
    async fn due(&self) -> Result<Vec<ScheduledMessage>> {
        let now = telebots_core::Time::now_secs();
        let due = self.store.due_reminders(now).await?;
        Ok(due
            .into_iter()
            .map(|reminder| ScheduledMessage {
                id: reminder.id,
                chat_id: reminder.chat_id,
                block: render::fired_reminder(&reminder.message),
            })
            .collect())
    }

    async fn delivered(&self, id: i64) -> Result<()> {
        self.store.delete_reminder(id).await
    }
}
