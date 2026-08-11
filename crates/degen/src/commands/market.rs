//! `/market` — global market overview (CMC global metrics).

use anyhow::Result;
use telebots_core::RenderBlock;
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util};

pub async fn handle(bot: Bot, msg: Message, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
async fn text(cmc: &CmcClient) -> Result<String> {
    Ok(cmc.global_metrics().await?.to_block().build())
}
