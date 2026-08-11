//! `/fear_greed` — the Fear & Greed index (CMC keyless public API).

use anyhow::Result;
use telebots_core::{Block, RenderBlock};

use crate::commands::Ctx;

/// The `/fear_greed` command.
pub struct FearGreed;

impl FearGreed {
    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        Ok(ctx.cmc.fear_greed().await?.to_block())
    }
}
