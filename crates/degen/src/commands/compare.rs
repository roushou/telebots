//! `/compare` — side-by-side quotes for two or more symbols.

use anyhow::{Result, bail};
use teloxide::prelude::*;

use crate::{cmc::CmcClient, commands::util, money::Money};

pub async fn handle(bot: Bot, msg: Message, args: String, cmc: CmcClient) -> ResponseResult<()> {
    util::send(bot, msg, text(&args, &cmc).await).await
}

/// Pure command logic; unit-testable without a bot or network.
pub async fn text(args: &str, cmc: &CmcClient) -> Result<String> {
    let symbols = util::normalize(args);
    if symbols.len() < 2 {
        bail!("Usage: /compare btc eth sol");
    }
    Ok(format_table(&cmc.quotes(&symbols).await?))
}

pub fn format_table(quotes: &[crate::cmc::Quote]) -> String {
    let rows: Vec<(String, String, String)> = quotes
        .iter()
        .map(|q| {
            let price = q
                .price
                .map(|p| Money::usd(p).to_string())
                .unwrap_or_else(|| "—".to_string());
            let change = q
                .change_24h
                .map(|c| {
                    let arrow = if c >= 0.0 { "▲" } else { "▼" };
                    format!("{arrow} {c:+.2}%")
                })
                .unwrap_or_else(|| "—".into());
            (q.symbol.clone(), price, change)
        })
        .collect();

    let sym_w = rows.iter().map(|r| r.0.chars().count()).max().unwrap_or(0);
    let price_w = rows.iter().map(|r| r.1.chars().count()).max().unwrap_or(0);

    let mut out = format!(
        "📊 {}",
        rows.iter()
            .map(|r| r.0.as_str())
            .collect::<Vec<_>>()
            .join(" vs ")
    );
    for (sym, price, change) in rows {
        out.push_str(&format!("\n{sym:<sym_w$}  {price:>price_w$}  {change}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmc::Quote;

    fn quote(symbol: &str, price: f64, change: Option<f64>) -> Quote {
        Quote {
            id: Some(1),
            name: symbol.into(),
            symbol: symbol.into(),
            rank: None,
            price: Some(price),
            change_24h: change,
            market_cap: None,
            volume_24h: None,
        }
    }

    #[test]
    fn format_table_aligns() {
        let out = format_table(&[
            quote("BTC", 95_432.1, Some(1.23)),
            quote("ETH", 3_500.0, Some(-0.5)),
        ]);
        assert_eq!(
            out,
            "📊 BTC vs ETH\nBTC  $95,432.1  ▲ +1.23%\nETH     $3,500  ▼ -0.50%"
        );
    }
}
