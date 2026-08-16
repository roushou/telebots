//! The built-in `/help` command: the command list plus the built-in commands
//! and an optional bot-specific tail.

use telebots_core::Block;
use teloxide::{
    Bot as Api, RequestError,
    dispatching::{UpdateFilterExt as _, UpdateHandler},
    types::{Me, Message, Update},
};

use crate::{
    command::CommandSpec,
    dispatch::{Supervisor, dispatch},
    reply::Reply,
};

/// The `/help` branch: answer with the command list (and an optional
/// bot-specific tail block).
pub(crate) fn help_branch<C>(extra: Option<Block>) -> UpdateHandler<RequestError>
where
    C: CommandSpec,
{
    Update::filter_message()
        .filter_map(|msg: Message, me: Me| {
            let bot_name = me.user.username.as_deref()?;
            let text = msg.text()?;
            is_help(text, bot_name).then_some(())
        })
        .endpoint(move |msg: Message, bot: Api, supervisor: Supervisor| {
            let block = render_help::<C>(extra.as_ref());
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

/// `text` is a `/help` command addressed to this bot.
fn is_help(text: &str, bot_name: &str) -> bool {
    let (command, _args) = text.split_once(' ').unwrap_or((text, ""));
    let (command, mention) = command.split_once('@').unwrap_or((command, ""));
    command == "/help" && (mention.is_empty() || mention.eq_ignore_ascii_case(bot_name))
}

/// Render the command list, the built-in commands, and an optional tail.
fn render_help<C>(extra: Option<&Block>) -> Block
where
    C: CommandSpec,
{
    let mut block = Block::new();
    block.line(C::help());
    block.blank();
    block.line("/help — Show help");
    block.line("/stats — Show bot stats");
    if let Some(extra) = extra {
        block.blank();
        block.push_block(extra.clone());
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_command_detection() {
        assert!(is_help("/help", "bot"));
        assert!(is_help("/help@bot", "bot"));
        assert!(is_help("/help extra args", "bot"));
        assert!(!is_help("/help@other", "bot"));
        assert!(!is_help("/price", "bot"));
        assert!(!is_help("help", "bot"));
    }
}
