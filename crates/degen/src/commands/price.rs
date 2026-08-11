//! `/price` — latest quotes for one or more symbols.

use anyhow::{Result, bail};
use teloxide::prelude::*;

use crate::{
    cmc::{CmcClient, Quote},
    commands::util,
};

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let symbols = util::normalize(args);
    if symbols.is_empty() {
        bail!("Usage: /price btc eth sol");
    }
    Ok(format_quotes(&cmc.quotes(&symbols).await?))
}

/// One card per quote, blank-line separated.
fn format_quotes(quotes: &[Quote]) -> String {
    quotes
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote(symbol: &str, price: f64) -> Quote {
        Quote {
            id: Some(1),
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            rank: None,
            price: Some(price),
            change_24h: None,
            market_cap: None,
            volume_24h: None,
        }
    }

    #[test]
    fn format_joins_cards() {
        let out = format_quotes(&[quote("BTC", 95_000.0), quote("ETH", 3_500.0)]);
        assert_eq!(
            out,
            "💰 BTC (BTC)\nPrice: $95,000\n24h: —\n\n💰 ETH (ETH)\nPrice: $3,500\n24h: —"
        );
    }
}
