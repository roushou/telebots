//! Typed command arguments.
//!
//! The raw argument string is converted to typed values once, at the
//! command boundary; string handling never leaks past [`Symbols`].

use std::ops::Deref;

/// Whitespace-split, uppercased symbols from a command's raw argument string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbols(Vec<String>);

impl Symbols {
    /// Split on whitespace and uppercase (symbols are case-insensitive).
    pub fn parse(raw: &str) -> Self {
        Self(raw.split_whitespace().map(|s| s.to_uppercase()).collect())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for Symbols {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_uppercases() {
        let symbols = Symbols::parse("btc  eth sol");
        assert_eq!(
            &*symbols,
            &["BTC".to_string(), "ETH".to_string(), "SOL".to_string()]
        );
    }

    #[test]
    fn empty_input_yields_no_symbols() {
        assert!(Symbols::parse("").is_empty());
        assert!(Symbols::parse("   ").is_empty());
    }
}
