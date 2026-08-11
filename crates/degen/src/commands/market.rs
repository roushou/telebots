//! `/market` — global market overview (CMC global metrics).

use anyhow::Result;
use telebots_core::{Block, RenderBlock};

use crate::commands::Ctx;

/// The `/market` command.
pub struct Market;

impl Market {
    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        Ok(ctx.cmc.global_metrics().await?.to_block())
    }
}
