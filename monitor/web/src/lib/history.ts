import type { BotDetail, BotSnapshot } from "./api";
import { healthOf, type Health } from "./health";

/// How often the Rust monitor records a snapshot, in seconds.
export const POLL_INTERVAL_SECS = 30;

export type HealthSegment = { start: number; end: number; status: Health };

/// Aggregated health over a window.
export type HealthSummary = {
  /// Percentage of the window fully up.
  uptimePct: number;
  /// Seconds not fully up (degraded + down).
  downtimeSecs: number;
  /// Longest contiguous outage (degraded or down), in seconds.
  longestOutageSecs: number;
};

/// Snapshots needed to cover `hours` at the poll cadence.
export function hoursToLimit(hours: number): number {
  return Math.ceil((hours * 3600) / POLL_INTERVAL_SECS);
}

/// Collapse a snapshot history (newest-first from the API) into contiguous
/// up/down/degraded spans. `start`/`end` are epoch seconds; a span ends one
/// poll interval after its last snapshot.
export function toSegments(history: BotSnapshot[]): HealthSegment[] {
  const sorted = history.toSorted((a, b) => a.ts - b.ts);
  const segments: HealthSegment[] = [];
  for (const snap of sorted) {
    const status = healthOf(snap);
    const last = segments[segments.length - 1];
    if (last && last.status === status) {
      last.end = snap.ts + POLL_INTERVAL_SECS;
    } else {
      segments.push({ start: snap.ts, end: snap.ts + POLL_INTERVAL_SECS, status });
    }
  }
  return segments;
}

/// Aggregate a window's segments into an uptime/downtime summary.
export function summarizeHealth(segments: HealthSegment[]): HealthSummary {
  let ok = 0;
  let notOk = 0;
  let longest = 0;
  let run = 0;
  for (const s of segments) {
    const dur = s.end - s.start;
    if (s.status === "ok") {
      ok += dur;
      run = 0;
    } else {
      notOk += dur;
      run += dur;
      longest = Math.max(longest, run);
    }
  }
  const total = ok + notOk;
  return {
    uptimePct: total === 0 ? 100 : (ok / total) * 100,
    downtimeSecs: notOk,
    longestOutageSecs: longest,
  };
}

function downsample<T>(rows: T[], max: number): T[] {
  if (rows.length <= max) return rows;
  const stride = Math.ceil(rows.length / max);
  const out: T[] = [];
  for (let i = 0; i < rows.length; i += stride) out.push(rows[i]);
  const last = rows[rows.length - 1];
  if (out[out.length - 1] !== last) out.push(last);
  return out;
}

/// Reduce raw history into the detail view's chart and log payloads.
export function buildBotDetail(
  hours: number,
  history: BotSnapshot[],
  latest: BotSnapshot,
): BotDetail {
  const sorted = history.toSorted((a, b) => a.ts - b.ts);

  const jobs = downsample(
    sorted.map((s) => ({
      ts: s.ts,
      active: s.status?.jobs_active ?? null,
      failed: s.status?.jobs_failed_total ?? null,
    })),
    400,
  );

  const panics = downsample(
    sorted.map((s) => ({ ts: s.ts, value: s.status?.panics_total ?? null })),
    400,
  );

  const restarts: { ts: number }[] = [];
  let prevUptime: number | null = null;
  for (const s of sorted) {
    if (s.status) {
      if (prevUptime !== null && s.status.uptime_secs < prevUptime) {
        restarts.push({ ts: s.ts });
      }
      prevUptime = s.status.uptime_secs;
    }
  }

  // Merge consecutive identical errors into incidents; a long outage is one
  // row, not thousands.
  const errors: { ts: number; end: number; message: string }[] = [];
  for (const s of sorted) {
    if (s.error) {
      const last = errors[errors.length - 1];
      if (last && last.message === s.error) {
        last.end = s.ts + POLL_INTERVAL_SECS;
      } else {
        errors.push({
          ts: s.ts,
          end: s.ts + POLL_INTERVAL_SECS,
          message: s.error,
        });
      }
    }
  }

  return {
    latest,
    hours,
    segments: toSegments(history),
    jobs,
    panics,
    restarts,
    errors,
  };
}
