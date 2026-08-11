//! `/convert` — convert an amount between assets.

use anyhow::{Result, bail};
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util, money::Money};

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let tokens = util::tokens(args);
    let (Some(amount), Some(symbol)) = (
        tokens.first().and_then(|t| t.parse::<f64>().ok()),
        tokens.get(1),
    ) else {
        bail!("Usage: /convert 100 btc usd");
    };
    let to = tokens
        .get(2)
        .map(|t| t.to_uppercase())
        .unwrap_or_else(|| "USD".to_string());
    let converted = cmc.convert(amount, symbol, &to).await?;
    Ok(format_conversion(amount, symbol, converted, &to))
}

pub fn format_conversion(amount: f64, symbol: &str, converted: f64, to: &str) -> String {
    format!(
        "💱 {amount} {} = {}",
        symbol.to_uppercase(),
        Money::new(converted, to)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_conversion_card() {
        let out = format_conversion(100.0, "btc", 6_700_000.0, "usd");
        assert_eq!(out, "💱 100 BTC = $6,700,000");
    }
}
