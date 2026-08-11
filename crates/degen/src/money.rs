//! Money and currency rendering.
//!
//! Currencies are data: a closed table of the codes this bot can plausibly
//! display, each with its conventional symbol and minor-unit precision. The
//! CoinMarketCap API speaks raw currency codes, so [`Money`] keeps the code
//! string as its identity — arbitrary codes pass through unchanged — and
//! resolves display attributes from the table at render time.
//!
//! The public surface is deliberately small: [`Money`] for amounts and
//! [`Currency`] for the data table. The number-formatting engine underneath
//! is private to this module.

/// How a number is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Notation {
    /// Grouped digits with significant-figure precision ("95,432.1").
    Full,
    /// K/M/B/T suffixes for large magnitudes ("1.23T").
    Compact,
}

/// Significant-figure precision policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SigFigs(u8);

const SUFFIXES: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];

/// Format `value` per the given notation and precision policy, capping
/// fractional digits at `max_decimals` when set (a currency's minor-unit
/// convention, e.g. JPY renders with zero decimals).
///
/// Total for all inputs: non-finite values render as "—" (never panics), zero
/// as "0". Precision is significant-figure based, so a 6-sig-fig value keeps
/// the same relative precision at 0.0000123 and 95,432.1; values too small
/// for decimal rendering fall back to scientific notation.
fn fmt_number(value: f64, notation: Notation, digits: SigFigs, max_decimals: Option<u8>) -> String {
    if !value.is_finite() {
        return "—".to_string();
    }
    let mag = value.abs();
    if mag == 0.0 {
        return "0".to_string();
    }
    let sign = if value.is_sign_negative() { "-" } else { "" };
    match notation {
        Notation::Full => body(sign, mag, digits.0, max_decimals),
        Notation::Compact => {
            for (div, suffix) in SUFFIXES {
                if mag >= div {
                    return format!(
                        "{sign}{}{suffix}",
                        sig_figs(mag / div, digits.0, max_decimals)
                    );
                }
            }
            body(sign, mag, digits.0, max_decimals)
        }
    }
}

/// The digit string for `mag > 0` rounded to `digits` significant figures (at
/// most `max_decimals` fractional digits when set), trailing zeros trimmed.
fn sig_figs(mag: f64, digits: u8, max_decimals: Option<u8>) -> String {
    let digits = digits.clamp(1, 15) as i32;
    let exp = mag.log10().floor() as i32;
    let decimals = match max_decimals {
        Some(cap) => (digits - 1 - exp).min(cap as i32),
        None => digits - 1 - exp,
    };
    if decimals > 20 {
        // Too small for decimal rendering; fall back to scientific notation.
        let s = format!("{mag:.*e}", (digits - 1) as usize);
        let (mantissa, e) = s.split_once('e').expect("scientific format contains 'e'");
        return format!(
            "{}e{}",
            mantissa.trim_end_matches('0').trim_end_matches('.'),
            e
        );
    }
    let decimals = decimals.clamp(0, 20) as usize;
    let s = format!("{mag:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Apply thousand separators to a non-negative integer string.
fn group(int_part: &str) -> String {
    let mut out = String::with_capacity(int_part.len() + int_part.len() / 3);
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `sig_figs` output plus sign and thousand separators.
fn body(sign: &str, mag: f64, digits: u8, max_decimals: Option<u8>) -> String {
    let s = sig_figs(mag, digits, max_decimals);
    match s.split_once('.') {
        Some((int, frac)) => format!("{sign}{}.{frac}", group(int)),
        None => format!("{sign}{}", group(&s)),
    }
}

/// Display attributes of a known currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Currency {
    /// ISO 4217 or crypto ticker, uppercase.
    pub code: &'static str,
    /// Prefix symbol ("$", "€"). `None` for codes conventionally written
    /// with the code itself, e.g. "CHF 1,234.56".
    pub symbol: Option<&'static str>,
    /// Conventional maximum fractional digits (JPY/KRW: 0, most fiat: 2,
    /// KWD: 3). `None` for crypto, where the significant-figure policy
    /// governs.
    pub decimals: Option<u8>,
}

/// Known currencies. Unknown codes fall back to "CODE amount" with no
/// precision cap.
const CURRENCIES: &[Currency] = &[
    Currency {
        code: "USD",
        symbol: Some("$"),
        decimals: Some(2),
    },
    Currency {
        code: "EUR",
        symbol: Some("€"),
        decimals: Some(2),
    },
    Currency {
        code: "GBP",
        symbol: Some("£"),
        decimals: Some(2),
    },
    Currency {
        code: "JPY",
        symbol: Some("¥"),
        decimals: Some(0),
    },
    Currency {
        code: "CNY",
        symbol: Some("¥"),
        decimals: Some(2),
    },
    Currency {
        code: "KRW",
        symbol: Some("₩"),
        decimals: Some(0),
    },
    Currency {
        code: "INR",
        symbol: Some("₹"),
        decimals: Some(2),
    },
    Currency {
        code: "RUB",
        symbol: Some("₽"),
        decimals: Some(2),
    },
    Currency {
        code: "CHF",
        symbol: None,
        decimals: Some(2),
    },
    Currency {
        code: "CAD",
        symbol: Some("C$"),
        decimals: Some(2),
    },
    Currency {
        code: "AUD",
        symbol: Some("A$"),
        decimals: Some(2),
    },
    Currency {
        code: "KWD",
        symbol: None,
        decimals: Some(3),
    },
    Currency {
        code: "BTC",
        symbol: Some("₿"),
        decimals: None,
    },
    Currency {
        code: "ETH",
        symbol: Some("Ξ"),
        decimals: None,
    },
];

fn currency(code: &str) -> Option<&'static Currency> {
    CURRENCIES
        .iter()
        .find(|c| c.code.eq_ignore_ascii_case(code))
}

/// A monetary amount in a given currency.
///
/// The currency is identified by its code string — the form the CoinMarketCap
/// API speaks — so unknown codes pass through unchanged; known codes gain
/// their symbol and minor-unit precision cap from the [`CURRENCIES`] table at
/// render time. Non-finite amounts render as "—".
#[derive(Debug, Clone)]
pub struct Money {
    amount: f64,
    code: String,
    notation: Notation,
    digits: SigFigs,
}

impl Money {
    /// Full-notation amount (6 significant figures).
    pub fn new(amount: f64, code: impl Into<String>) -> Self {
        Self {
            amount,
            code: code.into().to_ascii_uppercase(),
            notation: Notation::Full,
            digits: SigFigs(6),
        }
    }

    pub fn usd(amount: f64) -> Self {
        Self::new(amount, "USD")
    }

    /// Compact-notation amount (K/M/B/T, 3 significant figures) — for market
    /// caps and volumes.
    pub fn compact_usd(amount: f64) -> Self {
        Self {
            amount,
            code: "USD".into(),
            notation: Notation::Compact,
            digits: SigFigs(3),
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.amount.is_finite() {
            return write!(f, "—");
        }
        let cur = currency(&self.code);
        let body = fmt_number(
            self.amount,
            self.notation,
            self.digits,
            cur.and_then(|c| c.decimals),
        );
        match cur.and_then(|c| c.symbol) {
            Some(sym) => write!(f, "{sym}{body}"),
            None => write!(f, "{} {body}", self.code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full(v: f64) -> String {
        fmt_number(v, Notation::Full, SigFigs(6), None)
    }
    fn compact(v: f64) -> String {
        fmt_number(v, Notation::Compact, SigFigs(3), None)
    }

    #[test]
    fn full_notation_groups_and_keeps_precision() {
        assert_eq!(full(95_432.10), "95,432.1");
        assert_eq!(full(1_000_000.0), "1,000,000");
        assert_eq!(full(999.99), "999.99");
        assert_eq!(full(4_032.55), "4,032.55");
        assert_eq!(full(12.345), "12.345");
    }

    #[test]
    fn small_values_keep_precision() {
        assert_eq!(full(0.0000123), "0.0000123");
        assert_eq!(full(0.5), "0.5");
        assert_eq!(full(0.0), "0");
        assert_eq!(full(1e-11), "0.00000000001");
        assert_eq!(full(0.999999), "0.999999");
        assert_eq!(full(0.9999999), "1"); // rounds to 6 sig figs
        assert_eq!(full(1.00001), "1.00001");
    }

    #[test]
    fn tiny_values_fall_back_to_scientific() {
        assert_eq!(full(1.5e-18), "1.5e-18");
    }

    #[test]
    fn non_finite_is_graceful() {
        assert_eq!(full(f64::NAN), "—");
        assert_eq!(full(f64::INFINITY), "—");
        assert_eq!(full(f64::NEG_INFINITY), "—");
    }

    #[test]
    fn negative_values() {
        assert_eq!(full(-1_234.5), "-1,234.5");
        assert_eq!(full(-0.5), "-0.5");
        assert_eq!(full(-0.0), "0");
    }

    #[test]
    fn compact_notation() {
        assert_eq!(compact(1.23e12), "1.23T");
        assert_eq!(compact(4.032e9), "4.03B");
        assert_eq!(compact(123.4e6), "123M");
        assert_eq!(compact(12_345.6), "12.3K");
        assert_eq!(compact(999.99), "1,000");
        assert_eq!(compact(1e12), "1T");
    }

    #[test]
    fn money_uses_currency_table() {
        assert_eq!(Money::usd(95_432.1).to_string(), "$95,432.1");
        assert_eq!(Money::new(1_234.56, "EUR").to_string(), "€1,234.56");
        assert_eq!(Money::new(1_234.56, "chf").to_string(), "CHF 1,234.56");
        assert_eq!(Money::compact_usd(1.23e12).to_string(), "$1.23T");
        assert_eq!(Money::usd(f64::NAN).to_string(), "—");
        assert_eq!(Money::usd(f64::INFINITY).to_string(), "—");
    }

    #[test]
    fn currency_precision_caps() {
        // JPY/KRW conventionally have no minor unit.
        assert_eq!(Money::new(95_432.1, "JPY").to_string(), "¥95,432");
        assert_eq!(Money::new(95_432.1, "KRW").to_string(), "₩95,432");
        // KWD uses three decimals, rendered with the code prefix.
        assert_eq!(Money::new(1.234, "KWD").to_string(), "KWD 1.234");
        // The USD cap (2) never fights significant figures: 1 decimal stays.
        assert_eq!(Money::usd(95_432.1).to_string(), "$95,432.1");
    }

    #[test]
    fn crypto_uses_significant_figures() {
        assert_eq!(Money::new(0.0000123, "BTC").to_string(), "₿0.0000123");
        assert_eq!(Money::new(95_432.1, "ETH").to_string(), "Ξ95,432.1");
    }

    #[test]
    fn unknown_codes_fall_back_to_code_prefix() {
        assert_eq!(Money::new(1_234.56, "XYZ").to_string(), "XYZ 1,234.56");
    }

    #[test]
    fn currency_lookup_is_case_insensitive() {
        assert_eq!(currency("usd").map(|c| c.code), Some("USD"));
        assert_eq!(currency("XYZ"), None);
    }
}
