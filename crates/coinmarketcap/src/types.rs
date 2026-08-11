//! Public CoinMarketCap data types and their block rendering.

use std::fmt::{self, Display};

use telebots_core::{Block, Cell, Change, Line, RenderBlock, money::Money};

/// A CoinMarketCap quote. Rendered as a Telegram message card via its
/// [`Display`] impl below.
#[derive(Debug, Clone)]
pub struct Quote {
    pub id: Option<u64>,
    pub name: String,
    pub symbol: String,
    pub rank: Option<u32>,
    pub price: Option<f64>,
    pub change_24h: Option<f64>,
    pub market_cap: Option<f64>,
    pub volume_24h: Option<f64>,
}

impl Quote {
    /// A comparison-table row: symbol, right-aligned price, 24h change.
    pub fn compare_row(&self) -> [Cell; 3] {
        let price = self
            .price
            .map(|p| Money::usd(p).to_string())
            .unwrap_or_else(|| "—".to_string());
        let change = self
            .change_24h
            .map(|c| Change::new(c).to_string())
            .unwrap_or_else(|| "—".to_string());
        [
            Cell::new(&self.symbol),
            Cell::right(price),
            Cell::new(change),
        ]
    }
}

impl RenderBlock for Quote {
    fn render_block(&self, out: &mut Block) {
        // The card body as lines; the rank suffix attaches to the last line.
        let mut lines = vec![format!("💰 {} ({})", self.name, self.symbol)];
        match self.price {
            Some(p) => lines.push(format!("Price: {}", Money::usd(p))),
            None => lines.push("Price: —".to_string()),
        }
        match self.change_24h {
            Some(c) => lines.push(format!("24h: {}", Change::new(c))),
            None => lines.push("24h: —".to_string()),
        }
        if let Some(mc) = self.market_cap {
            lines.push(format!("MCap: {}", Money::compact_usd(mc)));
        }
        if let Some(v) = self.volume_24h {
            lines.push(format!("Vol (24h): {}", Money::compact_usd(v)));
        }
        if let Some(rank) = self.rank
            && let Some(last) = lines.last_mut()
        {
            last.push_str(&format!(" · rank #{rank}"));
        }
        for line in lines {
            out.line(line);
        }
    }
}

impl Display for Quote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_block().build())
    }
}

/// Global market metrics (from `global-metrics/quotes/latest`).
#[derive(Debug, Clone)]
pub struct GlobalMetrics {
    pub total_market_cap: f64,
    pub change_24h: Option<f64>,
    pub btc_dominance: f64,
    pub eth_dominance: f64,
}

impl RenderBlock for GlobalMetrics {
    fn render_block(&self, out: &mut Block) {
        out.line("🌐 Market Overview");
        let cap = match self.change_24h {
            Some(c) => format!(
                "{} ({} 24h)",
                Money::compact_usd(self.total_market_cap),
                Change::new(c)
            ),
            None => Money::compact_usd(self.total_market_cap).to_string(),
        };
        out.kv("Total MCap", cap);
        out.kv("BTC dominance", format!("{:.1}%", self.btc_dominance));
        out.kv("ETH dominance", format!("{:.1}%", self.eth_dominance));
    }
}

/// Project metadata (from `cryptocurrency/info`).
#[derive(Debug, Clone)]
pub struct CoinInfo {
    pub name: String,
    pub symbol: String,
    pub category: String,
    pub website: Option<String>,
    pub description: String,
}

/// Cap project descriptions so replies stay comfortably under Telegram's
/// message limit.
const DESCRIPTION_LIMIT: usize = 300;

impl RenderBlock for CoinInfo {
    fn render_block(&self, out: &mut Block) {
        out.line(format!("ℹ️ {} ({})", self.name, self.symbol));
        if !self.category.is_empty() {
            out.kv("Category", &self.category);
        }
        if let Some(website) = &self.website {
            out.line(format!("🔗 {website}"));
        }
        if !self.description.is_empty() {
            let mut line = Line::text(self.description.clone());
            line.ellipsize(DESCRIPTION_LIMIT);
            out.push(line);
        }
    }
}

/// The Fear & Greed index, 0 (extreme fear) to 100 (extreme greed).
/// From CMC's keyless public API (`/public-api/v3/fear-and-greed/latest`).
#[derive(Debug, Clone)]
pub struct FearGreed {
    pub value: u8,
    pub classification: String,
}

impl RenderBlock for FearGreed {
    fn render_block(&self, out: &mut Block) {
        out.line(format!(
            "😱 Fear & Greed: {}/100 — {}",
            self.value, self.classification
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_display_card() {
        let q = Quote {
            id: Some(1),
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            rank: Some(1),
            price: Some(95_432.1),
            change_24h: Some(1.23),
            market_cap: Some(1.23e12),
            volume_24h: Some(45.6e9),
        };
        let s = q.to_string();
        assert!(s.contains("💰 Bitcoin (BTC)"));
        assert!(s.contains("Price: $95,432.1"));
        assert!(s.contains("24h: ▲ +1.23%"));
        assert!(s.contains("MCap: $1.23T"));
        assert!(s.contains("Vol (24h): $45.6B"));
        assert!(s.ends_with("rank #1"));
    }

    #[test]
    fn global_metrics_block() {
        let m = GlobalMetrics {
            total_market_cap: 2.61e12,
            change_24h: Some(1.42),
            btc_dominance: 58.7,
            eth_dominance: 10.4,
        };
        assert_eq!(
            m.to_block().build(),
            "🌐 Market Overview\nTotal MCap: $2.61T (▲ +1.42% 24h)\nBTC dominance: 58.7%\nETH dominance: 10.4%"
        );
    }

    #[test]
    fn fear_greed_block() {
        let fg = FearGreed {
            value: 29,
            classification: "Fear".into(),
        };
        assert_eq!(fg.to_block().build(), "😱 Fear & Greed: 29/100 — Fear");
    }

    #[test]
    fn coin_info_block() {
        let info = CoinInfo {
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            category: "coin".into(),
            website: Some("https://bitcoin.org".into()),
            description: "A peer-to-peer electronic cash system.".into(),
        };
        assert_eq!(
            info.to_block().build(),
            "ℹ️ Bitcoin (BTC)\nCategory: coin\n🔗 https://bitcoin.org\nA peer-to-peer electronic cash system."
        );
    }

    #[test]
    fn coin_info_truncates_long_description() {
        let info = CoinInfo {
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            category: "coin".into(),
            website: None,
            description: "x".repeat(DESCRIPTION_LIMIT + 50),
        };
        let out = info.to_block().build();
        let last = out.lines().last().unwrap();
        assert!(last.ends_with('…'));
        assert!(last.chars().count() <= DESCRIPTION_LIMIT + 1);
    }

    #[test]
    fn quote_compare_row() {
        let q = Quote {
            id: Some(1),
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            rank: None,
            price: Some(95_432.1),
            change_24h: Some(1.23),
            market_cap: None,
            volume_24h: None,
        };
        let row = q.compare_row();
        assert_eq!(row[0].text(), "BTC");
        assert_eq!(row[1].text(), "$95,432.1");
        assert_eq!(row[2].text(), "▲ +1.23%");
    }

    #[test]
    fn quote_display_missing_data() {
        let q = Quote {
            id: Some(1),
            name: "X".into(),
            symbol: "X".into(),
            rank: None,
            price: None,
            change_24h: None,
            market_cap: None,
            volume_24h: None,
        };
        assert_eq!(q.to_string(), "💰 X (X)\nPrice: —\n24h: —");
    }
}
