//! Tracing setup: env-filter with a per-service fallback and optional
//! JSON output. The panic hook lives in [`crate::App::run`], where the
//! panic counter exists.

use tracing_subscriber::EnvFilter;

/// One-shot process setup; call from `main` after loading `.env`.
pub struct Telemetry;

impl Telemetry {
    /// Install the tracing subscriber for `service`.
    ///
    /// Filter: `RUST_LOG` when set, else
    /// `"<service>=info,teloxide=warn,reqwest=warn"`. `TELEBOTS_LOG_JSON=1`
    /// switches to JSON output.
    pub fn init(service: &'static str) {
        let fallback = format!("{service}=info,botkit=info,panic=error,teloxide=warn,reqwest=warn");
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&fallback));

        if std::env::var("TELEBOTS_LOG_JSON").is_ok_and(|v| v == "1") {
            tracing_subscriber::fmt()
                .json()
                .with_env_filter(filter)
                .init();
        } else {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}
