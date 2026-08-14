//! The built-in `/stats` admin command: the bot's own runtime metrics.

use telebots_core::Block;
use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    types::{Me, Message, Update},
};

use crate::{
    dispatch::{Supervisor, dispatch},
    metrics::Health,
    reply::Reply,
};

/// The `/stats` branch: parse and answer with the current health snapshot.
pub(crate) fn stats_branch() -> UpdateHandler<RequestError> {
    Update::filter_message()
        .filter_map(|msg: Message, me: Me| {
            let bot_name = me.user.username.as_deref()?;
            let text = msg.text()?;
            is_stats(text, bot_name).then_some(())
        })
        .endpoint(|msg: Message, bot: Api, supervisor: Supervisor| {
            Box::pin(async move {
                let block = render(&supervisor.health());
                dispatch(
                    &bot,
                    msg.chat.id,
                    msg.id,
                    msg.from.as_ref().map(|u| u.id.0 as i64),
                    &supervisor,
                    async { Ok(Reply::text(block)) },
                )
                .await
            })
        })
}

/// `text` is a `/stats` command addressed to this bot.
fn is_stats(text: &str, bot_name: &str) -> bool {
    let (command, _args) = text.split_once(' ').unwrap_or((text, ""));
    let (command, mention) = command.split_once('@').unwrap_or((command, ""));
    command == "/stats" && (mention.is_empty() || mention.eq_ignore_ascii_case(bot_name))
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
    fn stats_command_detection() {
        assert!(is_stats("/stats", "bot"));
        assert!(is_stats("/stats@bot", "bot"));
        assert!(is_stats("/stats extra args", "bot"));
        assert!(!is_stats("/stats@other", "bot"));
        assert!(!is_stats("/price", "bot"));
        assert!(!is_stats("stats", "bot"));
    }

    #[test]
    fn duration_formatting() {
        assert_eq!(fmt_duration(59), "0m");
        assert_eq!(fmt_duration(5 * 60), "5m");
        assert_eq!(fmt_duration(3 * 3_600 + 5 * 60), "3h 5m");
        assert_eq!(fmt_duration(2 * 86_400 + 3_600), "2d 1h");
    }
}
