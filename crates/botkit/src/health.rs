//! The axum health/metrics server: `/healthz` (liveness for the Docker
//! healthcheck) and `/metrics` (status JSON polled by the monitor).

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use tokio::net::TcpListener;

use crate::metrics::{Health, Metrics};

/// The health/metrics HTTP server.
pub struct Server;

impl Server {
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
