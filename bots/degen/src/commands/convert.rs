//! `/convert` — convert an amount between assets.

use anyhow::{Result, bail};
use telebots_core::{Block, money::Money};

use crate::commands::Ctx;

/// Typed arguments for `/convert`.
#[derive(Debug, Clone)]
pub struct ConvertArgs {
    pub amount: f64,
    pub symbol: String,
    pub to: String,
}

impl ConvertArgs {
    /// Parse and validate `"100 btc usd"` (target defaults to USD).
    pub fn parse(raw: &str) -> Result<Self> {
        let tokens: Vec<&str> = raw.split_whitespace().collect();
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
        Ok(Self {
            amount,
            symbol: symbol.to_uppercase(),
            to,
        })
    }

    fn format_line(&self, converted: f64) -> String {
        format!(
            "💱 {} {} = {}",
            self.amount,
            self.symbol,
            Money::new(converted, &self.to)
        )
    }

    /// Produce the reply block.
    pub async fn reply(&self, ctx: &Ctx) -> Result<Block> {
        let converted = ctx.cmc.convert(self.amount, &self.symbol, &self.to).await?;
        let mut b = Block::new();
        b.line(self.format_line(converted));
        Ok(b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_requires_amount_and_symbol() {
        assert!(ConvertArgs::parse("").is_err());
        assert!(ConvertArgs::parse("100").is_err());
        assert!(ConvertArgs::parse("btc usd").is_err());
        assert!(ConvertArgs::parse("x btc").is_err());
    }

    #[test]
    fn parse_defaults_to_usd() -> anyhow::Result<()> {
        let args = ConvertArgs::parse("100 btc")?;
        assert_eq!(args.amount, 100.0);
        assert_eq!(args.symbol, "BTC");
        assert_eq!(args.to, "USD");
        Ok(())
    }

    #[test]
    fn format_line() -> anyhow::Result<()> {
        let args = ConvertArgs::parse("100 btc usd")?;
        assert_eq!(args.format_line(6_700_000.0), "💱 100 BTC = $6,700,000");
        Ok(())
    }
}
