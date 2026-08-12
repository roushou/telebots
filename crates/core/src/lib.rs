//! Shared core library for Telebots bots.
//!
//! Organized by concern: [`blocks`] holds the text-block model and rendering
//! used when bots send data to Telegram, [`money`] holds currency
//! formatting. Future shared modules (storage, ...) are added as siblings.

pub mod blocks;
pub mod money;

pub use blocks::{Align, Block, Cell, Change, Line, Render};
pub use money::{Currency, Money};
