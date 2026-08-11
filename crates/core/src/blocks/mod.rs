//! Telegram text-block rendering.
//!
//! A [`Block`] is an ordered set of [`Line`]s — plain text or table rows of
//! aligned [`Cell`]s. Layout decisions (column alignment, joining,
//! truncation) are deferred to render time, so rows align themselves without
//! hand-computed padding, and the same block can render as plain text or a
//! monospace code block (where column alignment holds on Telegram's
//! variable-width font).
//!
//! Data types render themselves via the [`RenderBlock`] trait:
//!
//! ```
//! use telebots_core::{Block, RenderBlock};
//!
//! struct Quote { symbol: &'static str, price: f64 }
//!
//! impl RenderBlock for Quote {
//!     fn render_block(&self, out: &mut Block) {
//!         out.line(format!("{} — ${}", self.symbol, self.price));
//!     }
//! }
//! ```

mod change;

use std::fmt;

pub use change::Change;
use unicode_width::UnicodeWidthStr;

/// Horizontal alignment of a [`Cell`] within its column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
    Center,
}

/// A table cell: text plus column alignment.
#[derive(Debug, Clone)]
pub struct Cell {
    text: String,
    align: Align,
}

impl Cell {
    /// A left-aligned cell.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: Align::Left,
        }
    }

    /// A left-aligned cell.
    pub fn left(text: impl Into<String>) -> Self {
        Self::new(text)
    }

    /// A right-aligned cell (for numbers).
    pub fn right(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: Align::Right,
        }
    }

    /// A centered cell.
    pub fn center(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            align: Align::Center,
        }
    }

    /// This cell's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Render this cell padded to `width` display columns per [`Align`].
    fn render_padded(&self, width: usize) -> String {
        let pad = width.saturating_sub(self.text.width());
        match self.align {
            Align::Left => format!("{}{}", self.text, " ".repeat(pad)),
            Align::Right => format!("{}{}", " ".repeat(pad), self.text),
            Align::Center => {
                let left = pad / 2;
                let right = pad - left;
                format!("{}{}{}", " ".repeat(left), self.text, " ".repeat(right))
            }
        }
    }
}

impl From<&str> for Cell {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

/// A single line of a [`Block`]: plain text, a table row, or a blank line.
#[derive(Debug, Clone)]
pub enum Line {
    /// A single line of text (heading, key-value, raw).
    Text(String),
    /// A table row; cells align across rows sharing the same column index.
    Row(Vec<Cell>),
    /// An empty separator line.
    Blank,
}

impl Line {
    /// A text line.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// A table row.
    pub fn row(cells: impl IntoIterator<Item = impl Into<Cell>>) -> Self {
        Self::Row(cells.into_iter().map(Into::into).collect())
    }

    /// A blank separator line.
    pub fn blank() -> Self {
        Self::Blank
    }

    /// Truncate this line to `max` display columns, appending `…`.
    pub fn ellipsize(&mut self, max: usize) {
        match self {
            Line::Text(text) => *text = Self::cut(text.clone(), max),
            Line::Row(cells) => {
                let joined = cells
                    .iter()
                    .map(|c| c.text.as_str())
                    .collect::<Vec<_>>()
                    .join("  ");
                *self = Line::Text(Self::cut(joined, max));
            }
            Line::Blank => {}
        }
    }

    /// Cut `text` to `max` display columns, appending `…` when truncated.
    fn cut(text: String, max: usize) -> String {
        if text.width() <= max {
            return text;
        }
        let keep = max.saturating_sub(1);
        let mut out = String::new();
        for ch in text.chars() {
            if out.width() + ch.to_string().width() > keep {
                break;
            }
            out.push(ch);
        }
        out.push('…');
        out
    }
}

/// An ordered document of lines, rendered to a [`String`].
///
/// Rows are column-aligned at render time; the same block can render plain
/// or monospace.
#[derive(Debug, Clone, Default)]
pub struct Block {
    lines: Vec<Line>,
}

impl Block {
    /// An empty block.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a text line.
    pub fn line(&mut self, text: impl Into<String>) -> &mut Self {
        self.lines.push(Line::Text(text.into()));
        self
    }

    /// Append a `"Key: value"` line.
    pub fn kv(&mut self, key: impl fmt::Display, value: impl fmt::Display) -> &mut Self {
        self.line(format!("{key}: {value}"))
    }

    /// Append a table row.
    pub fn row(&mut self, cells: impl IntoIterator<Item = impl Into<Cell>>) -> &mut Self {
        self.lines
            .push(Line::Row(cells.into_iter().map(Into::into).collect()));
        self
    }

    /// Append a blank separator line.
    pub fn blank(&mut self) -> &mut Self {
        self.lines.push(Line::Blank);
        self
    }

    /// Append a line directly (e.g. after [`Line::ellipsize`]).
    pub fn push(&mut self, line: Line) -> &mut Self {
        self.lines.push(line);
        self
    }

    /// Append every line of another block.
    pub fn push_block(&mut self, other: Block) -> &mut Self {
        self.lines.extend(other.lines);
        self
    }

    /// Render as plain text.
    pub fn build(&self) -> String {
        self.render(Render::Plain)
    }

    /// Render as a fenced monospace code block (keeps table alignment on
    /// Telegram's variable-width font).
    pub fn render_monospace(&self) -> String {
        self.render(Render::Monospace)
    }

    /// Render in the given mode.
    pub fn render(&self, mode: Render) -> String {
        // Column widths come from all rows, computed once.
        let mut widths: Vec<usize> = Vec::new();
        for line in &self.lines {
            if let Line::Row(cells) = line {
                for (i, cell) in cells.iter().enumerate() {
                    if widths.len() <= i {
                        widths.resize(i + 1, 0);
                    }
                    widths[i] = widths[i].max(cell.text.width());
                }
            }
        }

        let mut out = String::new();
        for line in &self.lines {
            match line {
                Line::Blank => out.push('\n'),
                Line::Text(text) => {
                    out.push_str(text);
                    out.push('\n');
                }
                Line::Row(cells) => {
                    for (i, cell) in cells.iter().enumerate() {
                        if i > 0 {
                            out.push_str("  ");
                        }
                        out.push_str(&cell.render_padded(widths[i]));
                    }
                    out.push('\n');
                }
            }
        }
        while out.ends_with('\n') {
            out.pop();
        }

        match mode {
            Render::Plain => out,
            Render::Monospace => format!("```\n{out}\n```"),
        }
    }

    /// Keep only the first lines that fit within `max` display columns.
    ///
    /// Whole lines are dropped from the tail (never cut in half); a trailing
    /// `…` marks dropped content. A single line longer than `max` is cut.
    pub fn truncate(mut self, max: usize) -> Block {
        if self.rendered_width() <= max {
            return self;
        }
        let mut dropped = 0;
        while self.rendered_width() > max && self.lines.len() > 1 {
            self.lines.pop();
            dropped += 1;
        }
        if self.rendered_width() > max {
            if let Some(first) = self.lines.first_mut() {
                first.ellipsize(max);
            }
        } else if dropped > 0 {
            self.lines.push(Line::text("…"));
        }
        self
    }

    /// Rendered width in display columns (plain mode).
    fn rendered_width(&self) -> usize {
        self.build().width()
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.build())
    }
}

/// Render modes for a [`Block`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Render {
    /// Plain text with two-space column gutters.
    Plain,
    /// A fenced ` ``` ` code block; monospace keeps table alignment.
    Monospace,
}

/// Types that can render themselves into a [`Block`].
///
/// Implement this for data types (quotes, metrics, ...) so commands can
/// compose them with [`Block::push_block`]:
///
/// ```
/// use telebots_core::{Block, RenderBlock};
///
/// # struct Price;
/// # impl RenderBlock for Price {
/// #     fn render_block(&self, out: &mut Block) { out.line("x"); }
/// # }
/// let mut b = Block::new();
/// b.push_block(Price.to_block());
/// ```
pub trait RenderBlock {
    /// Append this value's lines to `out`.
    fn render_block(&self, out: &mut Block);

    /// Render this value as a standalone block.
    fn to_block(&self) -> Block {
        let mut out = Block::new();
        self.render_block(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_lines_join_with_newlines() {
        let mut b = Block::new();
        b.line("hello").line("world");
        assert_eq!(b.build(), "hello\nworld");
    }

    #[test]
    fn kv_formats_key_value() {
        let mut b = Block::new();
        b.kv("Price", "$100");
        assert_eq!(b.build(), "Price: $100");
    }

    #[test]
    fn blank_lines_separate() {
        let mut b = Block::new();
        b.line("a").blank().line("b");
        assert_eq!(b.build(), "a\n\nb");
    }

    #[test]
    fn rows_align_by_column() {
        let mut b = Block::new();
        b.row([Cell::new("BTC"), Cell::right("$95,432.1")]);
        b.row([Cell::new("ETH"), Cell::right("$3,500")]);
        assert_eq!(b.build(), "BTC  $95,432.1\nETH     $3,500");
    }

    #[test]
    fn center_alignment_splits_padding() {
        let c = Cell::center("x");
        assert_eq!(c.render_padded(4), " x  ");
    }

    #[test]
    fn monospace_wraps_in_fence() {
        let mut b = Block::new();
        b.line("a").line("b");
        assert_eq!(b.render_monospace(), "```\na\nb\n```");
    }

    #[test]
    fn truncate_drops_tail_lines() {
        let mut b = Block::new();
        b.line("1234567890").line("abcdefghij").line("klmnopqrst");
        assert_eq!(b.truncate(15).build(), "1234567890\n…");
    }

    #[test]
    fn truncate_hard_cuts_an_oversized_single_line() {
        let mut b = Block::new();
        b.line("abcdefghij");
        assert_eq!(b.truncate(5).build(), "abcd…");
    }

    #[test]
    fn truncate_keeps_blocks_that_fit() {
        let mut b = Block::new();
        b.line("abc").line("def");
        assert_eq!(b.truncate(100).build(), "abc\ndef");
    }

    #[test]
    fn ellipsize_line() {
        let mut l = Line::text("abcdefghij");
        l.ellipsize(5);
        let mut b = Block::new();
        b.push(l);
        assert_eq!(b.build(), "abcd…");
    }

    #[test]
    fn render_block_composes() {
        struct Price(f64);
        impl RenderBlock for Price {
            fn render_block(&self, out: &mut Block) {
                out.kv("Price", self.0);
            }
        }

        let mut b = Block::new();
        b.push_block(Price(95_000.0).to_block());
        assert_eq!(b.build(), "Price: 95000");
    }

    #[test]
    fn change_renders_arrow_and_sign() {
        assert_eq!(Change::new(1.23).to_string(), "▲ +1.23%");
        assert_eq!(Change::new(-0.5).to_string(), "▼ -0.50%");
        assert_eq!(Change::new(58.6).with_decimals(1).to_string(), "▲ +58.6%");
        assert_eq!(Change::new(0.0).to_string(), "▲ +0.00%");
    }
}
