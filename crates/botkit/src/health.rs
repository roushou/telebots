//! The axum health/metrics server: `/healthz` (liveness for the Docker
//! healthcheck) and `/metrics` (status JSON polled by the monitor).

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use tokio::net::TcpListener;

use crate::metrics::{Health, Metrics};

/// The health/metrics HTTP server.
pub struct Server;

impl Server {
    /// The port this service listens on: `TELEBOTS_METRICS_PORT` when set,
    /// else the per-service default (degen 9101, imagine 9102, else 9100).
    pub fn port_for(service: &str) -> u16 {
        if let Ok(raw) = std::env::var("TELEBOTS_METRICS_PORT")
            && let Ok(port) = raw.parse()
        {
            return port;
        }
        match service {
            "degen" => 9101,
            "imagine" => 9102,
            _ => 9100,
        }
    }

    /// Serve the health and metrics routes on `listener` in the background.
    pub fn serve(listener: TcpListener, metrics: Metrics) {
        let app = Router::new()
            .route("/healthz", get(healthz))
            .route("/metrics", get(status_json))
            .with_state(metrics);
        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                tracing::error!("metrics server failed: {e}");
            }
        });
    }
}

async fn healthz(State(metrics): State<Metrics>) -> (StatusCode, Json<Health>) {
    let health = metrics.health();
    let status = if metrics.alive() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(health))
}

async fn status_json(State(metrics): State<Metrics>) -> Json<Health> {
    Json(metrics.health())
}
