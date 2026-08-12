//! The shared bot shell: config loading, tracing setup, and the
//! dispatcher runner every Telebots bot is made of.

pub mod app;
pub mod config;
pub mod health;
pub mod metrics;
pub mod reply;
pub mod telemetry;

pub use app::{App, AppError};
pub use config::{ConfigError, Env, Key};
pub use metrics::Metrics;
pub use reply::{Job, JobCtx, Reply, Runtime, Supervisor, dispatch};
pub use telemetry::Telemetry;
