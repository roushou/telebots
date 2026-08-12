//! The axum JSON API.

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::db::Db;

/// How many history rows a request may ask for at most.
/// 24h at the 30s poll cadence is 2880 snapshots; leave headroom.
const MAX_HISTORY: usize = 5000;

/// Serve the JSON API.
pub async fn serve(port: u16, db: Db) -> Result<()> {
    let app = Router::new()
        .route("/api/bots", get(bots))
        .route("/api/bots/{name}/history", get(history))
        .route("/healthz", get(healthz))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("monitor on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}

/// Newest snapshot per bot.
async fn bots(State(db): State<Db>) -> Json<Value> {
    let snapshots = db.latest_per_bot().await.unwrap_or_default();
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
    State(db): State<Db>,
    Path(name): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> (StatusCode, Json<Value>) {
    let limit = query.limit.unwrap_or(100).min(MAX_HISTORY);
    match db.history(&name, limit).await {
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

async fn healthz() -> Json<Value> {
    Json(json!({ "service": "monitor", "ok": true }))
}
