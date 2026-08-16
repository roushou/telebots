//! The built-in `/stats` command: the bot's own runtime metrics.

use telebots_core::Block;
use teloxide::{RequestError, dispatching::UpdateHandler};

use crate::{builtin::builtin_branch, runtime::Health};

/// The `/stats` branch: answer with the current health snapshot.
pub(crate) fn stats_branch() -> UpdateHandler<RequestError> {
    builtin_branch("/stats", |s| render(&s.health()))
}

/// Render a health snapshot as a text block.
fn render(health: &Health) -> Block {
    let mut block = Block::new();
    block.line(format!("📊 {} v{}", health.service, health.version));
    block.kv("uptime", fmt_duration(health.uptime_secs));
    block.kv("telegram", health.telegram);
    block.kv("commands", health.commands_total);
    block.kv(
        "jobs",
        format!(
            "{} active · {} failed",
            health.jobs_active, health.jobs_failed_total
        ),
    );
    block.kv("panics", health.panics_total);
    block
}

fn fmt_duration(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_formatting() {
        assert_eq!(fmt_duration(59), "0m");
        assert_eq!(fmt_duration(5 * 60), "5m");
        assert_eq!(fmt_duration(3 * 3_600 + 5 * 60), "3h 5m");
        assert_eq!(fmt_duration(2 * 86_400 + 3_600), "2d 1h");
    }
}
