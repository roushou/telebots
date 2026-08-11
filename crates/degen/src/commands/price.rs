//! `/price` — latest quotes for one or more symbols.

use anyhow::{Result, bail};
use telebots_core::{Block, RenderBlock};
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util};

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let symbols = util::normalize(args);
    if symbols.is_empty() {
        bail!("Usage: /price btc eth sol");
    }
    let quotes = cmc.quotes(&symbols).await?;

    let mut b = Block::new();
    for (i, q) in quotes.iter().enumerate() {
        if i > 0 {
            b.blank();
        }
        b.push_block(q.to_block());
    }
    Ok(b.build())
}
