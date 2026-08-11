//! `/market` — global market overview (CMC global metrics).

use anyhow::Result;
use teloxide::prelude::*;

use crate::{
    cmc::{CmcClient, GlobalMetrics},
    commands::util,
    money::Money,
};

pub async fn handle(bot: Bot, msg: Message, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(cmc: &CmcClient) -> Result<String> {
    Ok(format_market(&cmc.global_metrics().await?))
}

pub fn format_market(m: &GlobalMetrics) -> String {
    let mut out = format!(
        "🌐 Market Overview\nTotal MCap: {}",
        Money::compact_usd(m.total_market_cap)
    );
    if let Some(c) = m.change_24h {
        let arrow = if c >= 0.0 { "▲" } else { "▼" };
        out.push_str(&format!(" ({arrow} {c:+.2}% 24h)"));
    }
    out.push_str(&format!("\nBTC dominance: {:.1}%", m.btc_dominance));
    out.push_str(&format!("\nETH dominance: {:.1}%", m.eth_dominance));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_overview() {
        let m = GlobalMetrics {
            total_market_cap: 2.61e12,
            change_24h: Some(1.42),
            btc_dominance: 58.7,
            eth_dominance: 10.4,
        };
        assert_eq!(
            format_market(&m),
            "🌐 Market Overview\nTotal MCap: $2.61T (▲ +1.42% 24h)\nBTC dominance: 58.7%\nETH dominance: 10.4%"
        );
    }
}
