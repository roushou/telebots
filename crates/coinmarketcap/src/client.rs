//! The CoinMarketCap HTTP client: request plumbing, endpoint methods, and
//! the wire-format response types. Public data types live in [`types`].

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::types::{CoinInfo, FearGreed, GlobalMetrics, Quote};

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

// --- Wire-format response types -----------------------------------------

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

#[derive(Deserialize)]
struct FearGreedResponse {
    data: FearGreedData,
}

#[derive(Deserialize)]
struct FearGreedData {
    value: u8,
    value_classification: String,
}
