//! `/compare` — side-by-side quotes for two or more symbols.

use anyhow::{Result, bail};
use telebots_core::Block;
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util};

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let symbols = util::normalize(args);
    if symbols.len() < 2 {
        bail!("Usage: /compare btc eth sol");
    }
    let quotes = cmc.quotes(&symbols).await?;

    let mut b = Block::new();
    b.line(format!("📊 {}", symbols.join(" vs ")));
    for q in &quotes {
        b.row(q.compare_row());
    }
    Ok(b.build())
}
