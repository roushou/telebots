//! The shared bot shell: config loading, tracing setup, and the
//! dispatcher runner every Telebots bot is made of.

pub mod app;
pub mod config;
pub mod telemetry;

pub use app::App;
pub use config::{Env, Key};
pub use telemetry::Telemetry;
