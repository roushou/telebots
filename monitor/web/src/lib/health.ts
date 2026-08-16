import type { BotSnapshot } from "./api";

/// A bot's health, derived from its latest snapshot.
export type Health = "ok" | "degraded" | "down";

/// Classify a snapshot:
/// - `down` — the bot's `/metrics` didn't respond (or responded badly).
/// - `degraded` — it responded, but reports itself unhealthy.
/// - `ok` — responding and healthy.
///
/// The bot is authoritative: `healthy` comes straight from its payload, so
/// this mirrors the server-side classification without re-deriving it.
export function healthOf(snap: BotSnapshot): Health {
  if (snap.status === null || snap.error !== null) return "down";
  return snap.status.healthy === true ? "ok" : "degraded";
}
