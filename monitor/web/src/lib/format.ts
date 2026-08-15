/// Presentation helpers for time and counts. Plain functions, stateless.

export function fmtAgo(secs: number | null): string {
  if (secs === null) return "—";
  if (secs < 60) return `${secs}s ago`;
  const min = Math.floor(secs / 60);
  if (min < 60) return `${min}m ago`;
  const h = Math.floor(min / 60);
  return `${h}h ${min % 60}m ago`;
}

export function fmtUptime(secs: number): string {
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function fmtDuration(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  const min = Math.round(secs / 60);
  if (min < 60) return `${min}m`;
  const h = Math.floor(min / 60);
  return `${h}h ${min % 60}m`;
}

/// A percentage, with more precision when near 100%.
export function fmtPct(value: number): string {
  return `${value.toFixed(value >= 99.9 ? 2 : 1)}%`;
}

/// Compact "HH:MM" for chart ticks. Accepts epoch seconds or ms.
export function fmtClock(ts: number): string {
  const ms = ts < 1e12 ? ts * 1000 : ts;
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(ms));
}

/// "Mar 5, 14:30" for tooltips and timestamps. Accepts epoch seconds or ms.
export function fmtStamp(ts: number): string {
  const ms = ts < 1e12 ? ts * 1000 : ts;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(ms));
}

export function fmtCompact(n: number): string {
  return new Intl.NumberFormat(undefined, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(n);
}

/// Token counts use the same compact notation ("1.2K").
export function fmtTokens(n: number): string {
  return fmtCompact(n);
}

/// Cost in micro-USD (millionths of a dollar) to a readable USD string.
export function fmtCostUsd(microUsd: number): string {
  const usd = microUsd / 1_000_000;
  if (usd <= 0) return "$0";
  if (usd >= 1) return `$${usd.toFixed(2)}`;
  return `$${usd.toFixed(4)}`;
}
