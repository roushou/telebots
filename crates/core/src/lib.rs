//! Shared core library for Telebots bots.
//!
//! Organized by concern: [`blocks`] holds the text-block model and rendering
//! used when bots send data to Telegram. Future shared modules (money,
//! storage, ...) are added as siblings of `blocks`.

pub mod blocks;

pub use blocks::{Align, Block, Cell, Change, Line, Render, RenderBlock};
