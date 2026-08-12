//! `/market` — global market overview (CMC global metrics).

use anyhow::Result;
use telebots_core::Block;

use crate::{commands::Ctx, render};

/// The `/market` command.
pub struct Market;

impl Market {
    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        Ok(render::metrics_card(&ctx.cmc.global_metrics().await?))
    }
}
