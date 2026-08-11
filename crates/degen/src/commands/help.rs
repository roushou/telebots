//! `/help` — list commands, generated from the enum's descriptions.

use teloxide::{prelude::*, utils::command::BotCommands};

use super::Command;

pub async fn handle(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}
