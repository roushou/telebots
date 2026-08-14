//! The axum JSON API.

use std::time::Instant;

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{db::Db, stats::Stats};

/// How many history rows a request may ask for at most.
/// 24h at the 30s poll cadence is 2880 snapshots; leave headroom.
const MAX_HISTORY: usize = 5000;

/// Two missed poll cycles means the monitor itself is stale.
const STALE_AFTER_SECS: u64 = 60;

#[derive(Clone)]
struct AppState {
    db: Db,
    stats: Stats,
    started: Instant,
    bots_configured: usize,
}

/// Serve the JSON API.
pub async fn serve(port: u16, db: Db, stats: Stats, bots_configured: usize) -> Result<()> {
    let app = Router::new()
        .route("/api/bots", get(bots))
        .route("/api/bots/{name}/history", get(history))
        .route("/metrics", get(metrics))
        .route("/healthz", get(healthz))
        .with_state(AppState {
            db,
            stats,
            started: Instant::now(),
            bots_configured,
        });

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("monitor on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Newest snapshot per bot.
async fn bots(State(state): State<AppState>) -> Json<Value> {
    let snapshots = state.db.latest_per_bot().await.unwrap_or_default();
    let out: Vec<Value> = snapshots
        .iter()
        .map(|s| {
            json!({
                "bot": s.bot,
                "ts": s.ts,
                "status": s.status,
                "error": s.error,
            })
        })
        .collect();
    Json(json!(out))
}

#[derive(Deserialize)]
struct HistoryQuery {
    limit: Option<usize>,
}

/// Recent snapshots for one bot, newest first.
async fn history(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> (StatusCode, Json<Value>) {
    let limit = query.limit.unwrap_or(100).min(MAX_HISTORY);
    match state.db.history(&name, limit).await {
        Ok(snapshots) => {
            let out: Vec<Value> = snapshots
                .iter()
                .map(|s| {
                    json!({
                        "bot": s.bot,
                        "ts": s.ts,
                        "status": s.status,
                        "error": s.error,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!(out)))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        ),
    }
}

/// The monitor's own status.
async fn metrics(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "service": "monitor",
        "uptime_secs": state.started.elapsed().as_secs(),
        "bots_configured": state.bots_configured,
        "last_poll_ago_secs": state.stats.last_poll_ago_secs().await,
        "poll_errors_total": state.stats.poll_errors(),
        "snapshots_total": state.stats.snapshots(),
    }))
}

/// Liveness: the process is up and the poller has run recently.
async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    let stale = state
        .stats
        .last_poll_ago_secs()
        .await
        .is_some_and(|ago| ago > STALE_AFTER_SECS);
    let status = if stale {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    (status, Json(json!({ "service": "monitor", "ok": !stale })))
}
