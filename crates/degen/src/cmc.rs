//! CoinMarketCap API client (free tier endpoints only).

use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use telebots_core::{Block, Cell, Change, Line, RenderBlock};

use crate::money::Money;

const API_BASE: &str = "https://pro-api.coinmarketcap.com";
const DEFAULT_CONVERT: &str = "USD";

#[derive(Clone)]
pub struct CmcClient {
    http: reqwest::Client,
    api_key: String,
}

impl CmcClient {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .user_agent("degen-bot/0.1")
            .build()
            .expect("failed to build HTTP client");
        Self { http, api_key }
    }

    /// Fetch latest quotes for one or more symbols (e.g. `["BTC", "ETH"]`).
    pub async fn quotes(&self, symbols: &[String]) -> Result<Vec<Quote>> {
        let resp: QuotesResponse = self
            .get("/v1/cryptocurrency/quotes/latest")
            .query(&[
                ("symbol", symbols.join(",")),
                ("convert", DEFAULT_CONVERT.to_string()),
            ])
            .send()
            .await
            .context("CoinMarketCap request failed")?
            .error_for_status()
            .context("CoinMarketCap returned an error status")?
            .json()
            .await
            .context("failed to parse CoinMarketCap response")?;

        ensure_ok(resp.status)?;

        let mut quotes: Vec<Quote> = resp.data.into_values().map(Quote::from_entry).collect();
        quotes.sort_by_key(|q| q.rank.unwrap_or(u32::MAX));
        Ok(quotes)
    }

    /// Convert `amount` of `symbol` into `to` (e.g. 100 BTC -> USD).
    pub async fn convert(&self, amount: f64, symbol: &str, to: &str) -> Result<f64> {
        let resp: ConversionResponse = self
            .get("/v1/tools/price-conversion")
            .query(&[
                ("amount", amount.to_string()),
                ("symbol", symbol.to_uppercase()),
                ("convert", to.to_uppercase()),
            ])
            .send()
            .await
            .context("CoinMarketCap request failed")?
            .error_for_status()
            .context("CoinMarketCap returned an error status")?
            .json()
            .await
            .context("failed to parse CoinMarketCap response")?;

        ensure_ok(resp.status)?;

        let key = to.to_uppercase();
        resp.data
            .quote
            .get(&key)
            .map(|q| q.price)
            .with_context(|| format!("no quote for conversion target {key}"))
    }

    /// Global market metrics: total cap, BTC/ETH dominance, 24h change.
    pub async fn global_metrics(&self) -> Result<GlobalMetrics> {
        let resp: GlobalMetricsResponse = self
            .get("/v1/global-metrics/quotes/latest")
            .send()
            .await
            .context("CoinMarketCap request failed")?
            .error_for_status()
            .context("CoinMarketCap returned an error status")?
            .json()
            .await
            .context("failed to parse CoinMarketCap response")?;

        ensure_ok(resp.status)?;

        let usd = resp
            .data
            .quote
            .get(DEFAULT_CONVERT)
            .ok_or_else(|| anyhow::anyhow!("global metrics missing USD quote"))?;
        Ok(GlobalMetrics {
            total_market_cap: usd.total_market_cap,
            change_24h: usd.total_market_cap_yesterday_percentage_change,
            btc_dominance: resp.data.btc_dominance,
            eth_dominance: resp.data.eth_dominance,
        })
    }

    /// Metadata (category, description, website) for symbols. Resolves each
    /// symbol to a CMC id via `quotes/latest`, then fetches `/info`.
    pub async fn info(&self, symbols: &[String]) -> Result<Vec<CoinInfo>> {
        let quotes = self.quotes(symbols).await?;
        let ids: Vec<String> = quotes
            .iter()
            .filter_map(|q| q.id)
            .map(|id| id.to_string())
            .collect();
        if ids.is_empty() {
            bail!("no price data for the given symbols");
        }

        let resp: InfoResponse = self
            .get("/v1/cryptocurrency/info")
            .query(&[("id", ids.join(","))])
            .send()
            .await
            .context("CoinMarketCap request failed")?
            .error_for_status()
            .context("CoinMarketCap returned an error status")?
            .json()
            .await
            .context("failed to parse CoinMarketCap response")?;

        ensure_ok(resp.status)?;

        // Keep the ranked order from the quotes call.
        let mut out = Vec::new();
        for q in &quotes {
            if let Some(id) = q.id
                && let Some(entry) = resp.data.get(&id.to_string())
            {
                out.push(CoinInfo::from_entry(entry));
            }
        }
        Ok(out)
    }

    /// The Fear & Greed index, 0 (extreme fear) to 100 (extreme greed).
    ///
    /// CMC serves this on its keyless public API (no key, no header) at
    /// `/public-api/v3/fear-and-greed/latest`.
    pub async fn fear_greed(&self) -> Result<FearGreed> {
        let resp: FearGreedResponse = self
            .get_public("/public-api/v3/fear-and-greed/latest")
            .send()
            .await
            .context("CoinMarketCap request failed")?
            .error_for_status()
            .context("CoinMarketCap returned an error status")?
            .json()
            .await
            .context("failed to parse CoinMarketCap response")?;

        Ok(FearGreed {
            value: resp.data.value,
            classification: resp.data.value_classification,
        })
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{API_BASE}{path}"))
            .header("X-CMC_PRO_API_KEY", self.api_key.as_str())
            .header("Accept", "application/json")
    }

    /// Keyless requests must not send the API key header.
    fn get_public(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(format!("{API_BASE}{path}"))
            .header("Accept", "application/json")
    }
}

fn ensure_ok(status: ApiStatus) -> Result<()> {
    if status.error_code == 0 {
        Ok(())
    } else {
        bail!(
            "CoinMarketCap error {}: {}",
            status.error_code,
            status.error_message.unwrap_or_default()
        )
    }
}

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
    fn from_entry(entry: CmcEntry) -> Self {
        let usd = entry.quote.get(DEFAULT_CONVERT);
        Self {
            id: Some(entry.id),
            name: entry.name,
            symbol: entry.symbol,
            rank: entry.cmc_rank,
            price: usd.map(|q| q.price),
            change_24h: usd.and_then(|q| q.percent_change_24h),
            market_cap: usd.and_then(|q| q.market_cap),
            volume_24h: usd.and_then(|q| q.volume_24h),
        }
    }

    /// A comparison-table row: symbol, right-aligned price, 24h change.
    pub(crate) fn compare_row(&self) -> [Cell; 3] {
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

#[derive(Deserialize)]
struct ApiStatus {
    error_code: i32,
    error_message: Option<String>,
}

#[derive(Deserialize)]
struct QuotesResponse {
    status: ApiStatus,
    data: HashMap<String, CmcEntry>,
}

#[derive(Deserialize)]
struct CmcEntry {
    id: u64,
    name: String,
    symbol: String,
    cmc_rank: Option<u32>,
    quote: HashMap<String, UsdQuote>,
}

#[derive(Deserialize)]
struct UsdQuote {
    price: f64,
    percent_change_24h: Option<f64>,
    market_cap: Option<f64>,
    volume_24h: Option<f64>,
}

#[derive(Deserialize)]
struct ConversionResponse {
    status: ApiStatus,
    data: ConversionData,
}

#[derive(Deserialize)]
struct ConversionData {
    quote: HashMap<String, UsdQuote>,
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

#[derive(Deserialize)]
struct GlobalMetricsResponse {
    status: ApiStatus,
    data: GlobalMetricsData,
}

#[derive(Deserialize)]
struct GlobalMetricsData {
    btc_dominance: f64,
    eth_dominance: f64,
    quote: HashMap<String, GlobalUsdQuote>,
}

#[derive(Deserialize)]
struct GlobalUsdQuote {
    total_market_cap: f64,
    total_market_cap_yesterday_percentage_change: Option<f64>,
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

impl CoinInfo {
    fn from_entry(entry: &InfoEntry) -> Self {
        Self {
            name: entry.name.clone(),
            symbol: entry.symbol.clone(),
            category: entry.category.clone(),
            website: entry.urls.website.first().cloned(),
            description: entry.description.clone(),
        }
    }
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

#[derive(Deserialize)]
struct FearGreedResponse {
    data: FearGreedData,
}

#[derive(Deserialize)]
struct FearGreedData {
    value: u8,
    value_classification: String,
}

#[derive(Deserialize)]
struct InfoResponse {
    status: ApiStatus,
    data: HashMap<String, InfoEntry>,
}

#[derive(Deserialize)]
struct InfoEntry {
    name: String,
    symbol: String,
    category: String,
    description: String,
    urls: InfoUrls,
}

#[derive(Deserialize, Default)]
struct InfoUrls {
    website: Vec<String>,
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
