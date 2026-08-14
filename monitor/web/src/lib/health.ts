import type { BotSnapshot } from "./api";

/// A bot's health, derived from its latest snapshot.
export type Health = "ok" | "degraded" | "down";

/// A heartbeat older than this means the bot's Telegram link is considered
/// stale (matches botkit's liveness threshold).
const HEARTBEAT_STALE_SECS = 180;

/// Classify a snapshot:
/// - `down` — the bot's `/metrics` didn't respond (or responded badly).
/// - `degraded` — it responded, but its Telegram heartbeat is stale or
///   `telegram` reports unreachable.
/// - `ok` — responding and its Telegram link is healthy.
export function healthOf(snap: BotSnapshot): Health {
  if (snap.status === null || snap.error !== null) return "down";
  const stale =
    snap.status.last_heartbeat_ago_secs !== null &&
    snap.status.last_heartbeat_ago_secs > HEARTBEAT_STALE_SECS;
  if (snap.status.telegram !== "ok" || stale) return "degraded";
  return "ok";
}
