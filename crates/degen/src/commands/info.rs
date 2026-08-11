//! `/info` — project metadata (category, website, description) via CMC.

use anyhow::{Result, bail};
use teloxide::prelude::*;

use crate::{
    cmc::{CmcClient, CoinInfo},
    commands::util,
};

/// Cap the description so replies stay comfortably under Telegram's limit.
const DESCRIPTION_LIMIT: usize = 300;

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let symbols = util::normalize(args);
    if symbols.is_empty() {
        bail!("Usage: /info btc eth");
    }
    Ok(format_info(&cmc.info(&symbols).await?))
}

pub fn format_info(infos: &[CoinInfo]) -> String {
    infos
        .iter()
        .map(|i| {
            let mut lines = vec![format!("ℹ️ {} ({})", i.name, i.symbol)];
            if !i.category.is_empty() {
                lines.push(format!("Category: {}", i.category));
            }
            if let Some(w) = &i.website {
                lines.push(format!("🔗 {w}"));
            }
            if !i.description.is_empty() {
                let mut desc: String = i.description.chars().take(DESCRIPTION_LIMIT).collect();
                if i.description.chars().count() > DESCRIPTION_LIMIT {
                    desc.push('…');
                }
                lines.push(desc);
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(desc: &str) -> CoinInfo {
        CoinInfo {
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            category: "coin".into(),
            website: Some("https://bitcoin.org".into()),
            description: desc.into(),
        }
    }

    #[test]
    fn format_card() {
        let out = format_info(&[info("A peer-to-peer electronic cash system.")]);
        assert_eq!(
            out,
            "ℹ️ Bitcoin (BTC)\nCategory: coin\n🔗 https://bitcoin.org\nA peer-to-peer electronic cash system."
        );
    }

    #[test]
    fn truncates_long_descriptions() {
        let long = "x".repeat(DESCRIPTION_LIMIT + 50);
        let out = format_info(&[info(&long)]);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() < DESCRIPTION_LIMIT + 100);
    }
}
