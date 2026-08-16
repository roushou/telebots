//! Built-in slash commands: a shared branch builder plus the `/help` and
//! `/stats` commands.

mod help;
mod stats;

pub(crate) use help::help_branch;
pub(crate) use stats::stats_branch;
use telebots_core::Block;
use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    types::{Me, Message, Update},
};

use crate::{dispatch::dispatch, reply::Reply, supervisor::Supervisor};

/// Build a text-reply branch for a built-in slash command.
pub(crate) fn builtin_branch<F>(command: &'static str, render: F) -> UpdateHandler<RequestError>
where
    F: Fn(&Supervisor) -> Block + Send + Sync + 'static,
{
    Update::filter_message()
        .filter_map(move |msg: Message, me: Me| {
            let bot_name = me.user.username.as_deref()?;
            let text = msg.text()?;
            is_addressed(command, text, bot_name).then_some(())
        })
        .endpoint(move |msg: Message, bot: Api, supervisor: Supervisor| {
            let block = render(&supervisor);
            Box::pin(async move {
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

/// `text` is `command` addressed to this bot.
fn is_addressed(command: &str, text: &str, bot_name: &str) -> bool {
    let (head, _args) = text.split_once(' ').unwrap_or((text, ""));
    let (head, mention) = head.split_once('@').unwrap_or((head, ""));
    head == command && (mention.is_empty() || mention.eq_ignore_ascii_case(bot_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addressed_command_detection() {
        assert!(is_addressed("/stats", "/stats", "bot"));
        assert!(is_addressed("/stats", "/stats@bot", "bot"));
        assert!(is_addressed("/stats", "/stats extra args", "bot"));
        assert!(!is_addressed("/stats", "/stats@other", "bot"));
        assert!(!is_addressed("/stats", "/help", "bot"));
        assert!(!is_addressed("/stats", "stats", "bot"));
    }
}
