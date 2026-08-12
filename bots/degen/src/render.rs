//! Presentation: client-crate data rendered into `Block`s.

use coingecko::TrendingCoin;
use coinmarketcap::{CoinInfo, FearGreed, GlobalMetrics, Quote};
use telebots_core::{Block, Cell, Change, Line, Money};

/// Cap project descriptions so replies stay comfortably under Telegram's
/// message limit.
const DESCRIPTION_LIMIT: usize = 300;

/// A quote card: name, price, 24h change, market cap, volume, rank.
pub fn quote_card(q: &Quote) -> Block {
    // The card body as lines; the rank suffix attaches to the last line.
    let mut lines = vec![format!("💰 {} ({})", q.name, q.symbol)];
    match q.price {
        Some(p) => lines.push(format!("Price: {}", Money::usd(p))),
        None => lines.push("Price: —".to_string()),
    }
    match q.change_24h {
        Some(c) => lines.push(format!("24h: {}", Change::new(c))),
        None => lines.push("24h: —".to_string()),
    }
    if let Some(mc) = q.market_cap {
        lines.push(format!("MCap: {}", Money::compact_usd(mc)));
    }
    if let Some(v) = q.volume_24h {
        lines.push(format!("Vol (24h): {}", Money::compact_usd(v)));
    }
    if let Some(rank) = q.rank
        && let Some(last) = lines.last_mut()
    {
        last.push_str(&format!(" · rank #{rank}"));
    }
    let mut b = Block::new();
    for line in lines {
        b.line(line);
    }
    b
}

/// A comparison-table row: symbol, right-aligned price, 24h change.
pub fn quote_row(q: &Quote) -> [Cell; 3] {
    let price = q
        .price
        .map(|p| Money::usd(p).to_string())
        .unwrap_or_else(|| "—".to_string());
    let change = q
        .change_24h
        .map(|c| Change::new(c).to_string())
        .unwrap_or_else(|| "—".to_string());
    [Cell::new(&q.symbol), Cell::right(price), Cell::new(change)]
}

/// Global market overview: total cap, BTC/ETH dominance.
pub fn metrics_card(m: &GlobalMetrics) -> Block {
    let mut b = Block::new();
    b.line("🌐 Market Overview");
    let cap = match m.change_24h {
        Some(c) => format!(
            "{} ({} 24h)",
            Money::compact_usd(m.total_market_cap),
            Change::new(c)
        ),
        None => Money::compact_usd(m.total_market_cap).to_string(),
    };
    b.kv("Total MCap", cap);
    b.kv("BTC dominance", format!("{:.1}%", m.btc_dominance));
    b.kv("ETH dominance", format!("{:.1}%", m.eth_dominance));
    b
}

/// Project info card: category, website, truncated description.
pub fn coin_info_card(info: &CoinInfo) -> Block {
    let mut b = Block::new();
    b.line(format!("ℹ️ {} ({})", info.name, info.symbol));
    if !info.category.is_empty() {
        b.kv("Category", &info.category);
    }
    if let Some(website) = &info.website {
        b.line(format!("🔗 {website}"));
    }
    if !info.description.is_empty() {
        let mut line = Line::text(info.description.clone());
        line.ellipsize(DESCRIPTION_LIMIT);
        b.push(line);
    }
    b
}

/// The Fear & Greed index, 0 (extreme fear) to 100 (extreme greed).
pub fn fear_greed_card(fg: &FearGreed) -> Block {
    let mut b = Block::new();
    b.line(format!(
        "😱 Fear & Greed: {}/100 — {}",
        fg.value, fg.classification
    ));
    b
}

/// One trending-coin line.
pub fn trending_line(coin: &TrendingCoin) -> String {
    let rank = coin
        .market_cap_rank
        .map(|r| format!("#{r} "))
        .unwrap_or_default();
    let change = coin
        .change_24h
        .map(|c| format!(" {}", Change::new(c).with_decimals(1)))
        .unwrap_or_default();
    format!("{rank}{} ({}){change}", coin.name, coin.symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quote() -> Quote {
        Quote {
            id: Some(1),
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            rank: Some(1),
            price: Some(95_432.1),
            change_24h: Some(1.23),
            market_cap: Some(1.23e12),
            volume_24h: Some(45.6e9),
        }
    }

    #[test]
    fn quote_card_full() {
        let s = quote_card(&quote()).build();
        assert!(s.contains("💰 Bitcoin (BTC)"));
        assert!(s.contains("Price: $95,432.1"));
        assert!(s.contains("24h: ▲ +1.23%"));
        assert!(s.contains("MCap: $1.23T"));
        assert!(s.contains("Vol (24h): $45.6B"));
        assert!(s.ends_with("rank #1"));
    }

    #[test]
    fn quote_card_missing_data() {
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
        assert_eq!(quote_card(&q).build(), "💰 X (X)\nPrice: —\n24h: —");
    }

    #[test]
    fn quote_row_formats_cells() {
        let row = quote_row(&quote());
        assert_eq!(row[0].text(), "BTC");
        assert_eq!(row[1].text(), "$95,432.1");
        assert_eq!(row[2].text(), "▲ +1.23%");
    }

    #[test]
    fn quote_row_missing_data() {
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
        let row = quote_row(&q);
        assert_eq!(row[0].text(), "X");
        assert_eq!(row[1].text(), "—");
        assert_eq!(row[2].text(), "—");
    }

    #[test]
    fn metrics_card_block() {
        let m = GlobalMetrics {
            total_market_cap: 2.61e12,
            change_24h: Some(1.42),
            btc_dominance: 58.7,
            eth_dominance: 10.4,
        };
        assert_eq!(
            metrics_card(&m).build(),
            "🌐 Market Overview\nTotal MCap: $2.61T (▲ +1.42% 24h)\nBTC dominance: 58.7%\nETH dominance: 10.4%"
        );
    }

    #[test]
    fn fear_greed_card_block() {
        let fg = FearGreed {
            value: 29,
            classification: "Fear".into(),
        };
        assert_eq!(
            fear_greed_card(&fg).build(),
            "😱 Fear & Greed: 29/100 — Fear"
        );
    }

    #[test]
    fn coin_info_card_block() {
        let info = CoinInfo {
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            category: "coin".into(),
            website: Some("https://bitcoin.org".into()),
            description: "A peer-to-peer electronic cash system.".into(),
        };
        assert_eq!(
            coin_info_card(&info).build(),
            "ℹ️ Bitcoin (BTC)\nCategory: coin\n🔗 https://bitcoin.org\nA peer-to-peer electronic cash system."
        );
    }

    #[test]
    fn coin_info_card_truncates_long_description() {
        let info = CoinInfo {
            name: "Bitcoin".into(),
            symbol: "BTC".into(),
            category: "coin".into(),
            website: None,
            description: "x".repeat(DESCRIPTION_LIMIT + 50),
        };
        let out = coin_info_card(&info).build();
        let last = out.lines().last().unwrap();
        assert!(last.ends_with('…'));
        assert!(last.chars().count() <= DESCRIPTION_LIMIT + 1);
    }

    #[test]
    fn trending_line_with_rank_and_change() {
        let coin = TrendingCoin {
            name: "Velvet".into(),
            symbol: "VELVET".into(),
            market_cap_rank: Some(124),
            change_24h: Some(58.6),
        };
        assert_eq!(trending_line(&coin), "#124 Velvet (VELVET) ▲ +58.6%");
    }

    #[test]
    fn trending_line_without_rank_or_change() {
        let coin = TrendingCoin {
            name: "Pons".into(),
            symbol: "PONS".into(),
            market_cap_rank: None,
            change_24h: None,
        };
        assert_eq!(trending_line(&coin), "Pons (PONS)");
    }
}
