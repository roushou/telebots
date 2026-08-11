//! Shared helpers for command handlers: argument parsing and uniform reply
//! formatting. Keeps per-command modules small and the bot's UX consistent.

use anyhow::Result;
use teloxide::prelude::*;

/// Splits a raw argument string on whitespace.
pub fn tokens(args: &str) -> Vec<&str> {
    args.split_whitespace().collect()
}

/// Tokens, uppercased (symbols are case-insensitive).
pub fn normalize(args: &str) -> Vec<String> {
    tokens(args).into_iter().map(|s| s.to_uppercase()).collect()
}

/// Replies with a command's `Result<String>`: `Ok` text as-is, `Err` as a
/// uniform `⚠️` message. This is the one place handlers touch `send_message`.
pub async fn send(bot: Bot, msg: Message, result: Result<String>) -> ResponseResult<()> {
    let text = match result {
        Ok(text) => text,
        Err(e) => format!("⚠️ {e:#}"),
    };
    bot.send_message(msg.chat.id, text).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_split_on_whitespace() {
        assert_eq!(tokens("btc  eth  sol"), ["btc", "eth", "sol"]);
        assert!(tokens("").is_empty());
    }

    #[test]
    fn normalize_uppercases() {
        assert_eq!(normalize("bTc eTh"), ["BTC", "ETH"]);
    }
}
