//! `/fear_greed` — the Fear & Greed index (CMC keyless public API).

use anyhow::Result;
use telebots_core::Block;

use crate::{commands::Ctx, render};

/// The `/fear_greed` command.
pub struct FearGreed;

impl FearGreed {
    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        Ok(render::fear_greed_card(&ctx.cmc.fear_greed().await?))
    }
}
