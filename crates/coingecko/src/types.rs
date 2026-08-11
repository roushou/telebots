//! Public CoinGecko data types and their block rendering.

use telebots_core::{Block, Change, RenderBlock};

/// One trending coin.
#[derive(Debug, Clone)]
pub struct TrendingCoin {
    pub name: String,
    pub symbol: String,
    pub market_cap_rank: Option<u32>,
    /// 24h percent change in USD.
    pub change_24h: Option<f64>,
}

impl RenderBlock for TrendingCoin {
    fn render_block(&self, out: &mut Block) {
        let rank = self
            .market_cap_rank
            .map(|r| format!("#{r} "))
            .unwrap_or_default();
        let change = self
            .change_24h
            .map(|c| format!(" {}", Change::new(c).with_decimals(1)))
            .unwrap_or_default();
        out.line(format!("{rank}{} ({}){change}", self.name, self.symbol));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trending_entry_block() {
        let coin = TrendingCoin {
            name: "Velvet".into(),
            symbol: "VELVET".into(),
            market_cap_rank: Some(124),
            change_24h: Some(58.6),
        };
        assert_eq!(coin.to_block().build(), "#124 Velvet (VELVET) ▲ +58.6%");
    }

    #[test]
    fn trending_entry_without_rank_or_change() {
        let coin = TrendingCoin {
            name: "Pons".into(),
            symbol: "PONS".into(),
            market_cap_rank: None,
            change_24h: None,
        };
        assert_eq!(coin.to_block().build(), "Pons (PONS)");
    }
}
