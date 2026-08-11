//! Value formatters used inside blocks.

use std::fmt;

/// A signed percentage change, rendered with a direction arrow.
///
/// ```
/// use telebots_core::Change;
///
/// assert_eq!(Change::new(1.23).to_string(), "▲ +1.23%");
/// assert_eq!(Change::new(-0.5).to_string(), "▼ -0.50%");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Change {
    value: f64,
    decimals: u8,
}

impl Change {
    /// A change with two decimals.
    pub fn new(value: f64) -> Self {
        Self { value, decimals: 2 }
    }

    /// Set the number of decimals to render (default 2).
    pub fn with_decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let arrow = if self.value >= 0.0 { "▲" } else { "▼" };
        write!(
            f,
            "{arrow} {:+.decimals$}%",
            self.value,
            decimals = self.decimals as usize
        )
    }
}
