//! Presentation: reminders and times rendered into `Block`s.

use telebots_core::Block;
use time::{OffsetDateTime, UtcOffset};

use crate::store::Reminder;

/// The `when` grammar summary shown after the built-in `/help` list.
pub fn when_help() -> Block {
    let mut b = Block::new();
    b.line("When to remind — set with /remind <when> <message>:");
    b.row(["in 15m / 2h30m / 1d", "relative time"]);
    b.row(["9am / 14:30 / noon", "today (or tomorrow if past)"]);
    b.row(["monday / next monday", "day of week"]);
    b.row(["june 5 / 5 june", "month and day"]);
    b.row(["tomorrow 9am / 9am tomorrow", "date + time (either order)"]);
    b.row(["2025-06-01 09:00", "an exact date"]);
    b.blank();
    b.line("Set your zone with /timezone <offset> (e.g. +2, -5, +5:30).");
    b
}

/// The confirmation sent right after a reminder is set.
pub fn reminder_confirmed(at_secs: i64, message: &str, offset_minutes: i16) -> Block {
    let mut b = Block::new();
    b.line(format!(
        "⏰ I'll remind you \"{message}\" on {} ({}).",
        format_fire_at(at_secs, offset_minutes),
        format_offset(offset_minutes),
    ));
    b
}

/// The message delivered when a reminder fires.
pub fn fired_reminder(message: &str) -> Block {
    let mut b = Block::new();
    b.line(format!("⏰ Reminder: {message}"));
    b
}

/// A chat's pending reminders, soonest first.
pub fn list_reminders(reminders: &[Reminder], offset_minutes: i16) -> Block {
    let mut b = Block::new();
    if reminders.is_empty() {
        b.line("No upcoming reminders — set one with /remind in 15m buy milk");
        return b;
    }
    b.line(format!("⏰ Upcoming reminders ({}):", reminders.len()));
    for reminder in reminders {
        b.line(format!(
            "{}. {} — {}",
            reminder.id,
            reminder.message,
            format_fire_at(reminder.fire_at, offset_minutes),
        ));
    }
    b.line("Cancel with /cancel <number>.");
    b
}

/// The confirmation after a `/timezone` change.
pub fn timezone_set(offset_minutes: i16) -> Block {
    let mut b = Block::new();
    b.line(format!(
        "🕐 Timezone set to {}.",
        format_offset(offset_minutes)
    ));
    b
}

/// `2025-06-01 09:00` in the chat's offset.
fn format_fire_at(at_secs: i64, offset_minutes: i16) -> String {
    let offset = offset(offset_minutes);
    let dt = OffsetDateTime::from_unix_timestamp(at_secs)
        .unwrap_or(OffsetDateTime::UNIX_EPOCH)
        .to_offset(offset);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        dt.year(),
        dt.month() as u8,
        dt.day(),
        dt.hour(),
        dt.minute(),
    )
}

/// `UTC+02:00` / `UTC-05:30` from a signed minute offset.
fn format_offset(minutes: i16) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let abs = i16::abs(minutes);
    format!("UTC{sign}{:02}:{:02}", abs / 60, abs % 60)
}

/// A validated `UtcOffset` from a signed minute offset (falls back to UTC).
fn offset(minutes: i16) -> UtcOffset {
    UtcOffset::from_whole_seconds(i32::from(minutes) * 60).unwrap_or(UtcOffset::UTC)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reminder(id: i64, fire_at: i64, message: &str) -> Reminder {
        Reminder {
            id,
            chat_id: 1,
            fire_at,
            message: message.into(),
        }
    }

    #[test]
    fn fired_reminder_prepends_label() {
        assert_eq!(fired_reminder("buy milk").build(), "⏰ Reminder: buy milk");
    }

    #[test]
    fn empty_list_has_hint() {
        assert!(
            list_reminders(&[], 0)
                .build()
                .contains("No upcoming reminders")
        );
    }

    #[test]
    fn list_shows_id_message_and_time() {
        let out = list_reminders(&[reminder(3, 1_748_779_200, "buy milk")], 0).build();
        assert!(out.contains("3. buy milk"));
        assert!(out.contains("2025-06-01"));
        assert!(out.contains("Cancel with /cancel <number>"));
    }

    #[test]
    fn confirmation_includes_time_and_offset() {
        let out = reminder_confirmed(1_748_779_200, "buy milk", 120).build();
        assert!(out.contains("\"buy milk\""));
        assert!(out.contains("2025-06-01"));
        assert!(out.contains("UTC+02:00"));
    }

    #[test]
    fn timezone_set_formats_offsets() {
        assert_eq!(timezone_set(0).build(), "🕐 Timezone set to UTC+00:00.");
        assert_eq!(timezone_set(-330).build(), "🕐 Timezone set to UTC-05:30.");
        assert_eq!(timezone_set(120).build(), "🕐 Timezone set to UTC+02:00.");
    }
}
