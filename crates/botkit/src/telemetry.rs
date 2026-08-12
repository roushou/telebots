//! Tracing setup: env-filter with a per-service fallback, optional JSON
//! output, and a panic hook.

use std::panic;

use tracing_subscriber::EnvFilter;

/// One-shot process setup; call from `main` after loading `.env`.
pub struct Telemetry;

impl Telemetry {
    /// Install the tracing subscriber and panic hook for `service`.
    ///
    /// Filter: `RUST_LOG` when set, else
    /// `"<service>=info,teloxide=warn,reqwest=warn"`. `TELEBOTS_LOG_JSON=1`
    /// switches to JSON output.
    pub fn init(service: &'static str) {
        let fallback = format!("{service}=info,teloxide=warn,reqwest=warn");
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

        Self::install_panic_hook();
    }

    fn install_panic_hook() {
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            default_hook(info);
            let location = info.location().map(|l| l.to_string());
            let message = info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
                .unwrap_or("(no message)");
            tracing::error!(target: "panic", location, "panicked: {message}");
        }));
    }
}
