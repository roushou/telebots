//! The built-in `/help` command: the command list plus the built-in commands
//! and an optional bot-specific tail.

use telebots_core::Block;
use teloxide::{RequestError, dispatching::UpdateHandler};

use crate::{builtin::builtin_branch, handlers::CommandSpec};

/// The `/help` branch: answer with the command list (and an optional
/// bot-specific tail block).
pub(crate) fn help_branch<C>(extra: Option<Block>) -> UpdateHandler<RequestError>
where
    C: CommandSpec,
{
    builtin_branch("/help", move |_s| render_help::<C>(extra.as_ref()))
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
