//! The shared bot shell: config loading, tracing setup, and the
//! dispatcher runner every Telebots bot is made of.

pub mod app;
pub mod config;
pub mod reply;
pub mod telemetry;

pub use app::App;
pub use config::{Env, Key};
pub use reply::{Job, JobCtx, Reply, Supervisor, dispatch};
pub use telemetry::Telemetry;
