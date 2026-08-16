import { createServerFn } from "@tanstack/react-start";

import { type HealthSegment, buildBotDetail, hoursToLimit, toSegments } from "./history";

export type BotStatus = {
  service: string;
  version: string;
  uptime_secs: number;
  telegram: string;
  /// Whether the bot reports itself healthy (authoritative).
  healthy?: boolean;
  last_heartbeat_ago_secs: number | null;
  last_command_ago_secs: number | null;
  commands_total: number;
  dispatch_errors_total: number;
  jobs_active: number;
  jobs_failed_total: number;
  panics_total: number;
  /// Per-command counters (present once botkit exposes them).
  commands?: Record<string, { total: number; errors: number }>;
  /// LLM token usage and cost counters (present once botkit exposes them).
  llm_prompt_tokens_total?: number;
  llm_completion_tokens_total?: number;
  llm_requests_total?: number;
  llm_cost_micro_usd_total?: number;
};

export type BotSnapshot = {
  bot: string;
  ts: number;
  status: BotStatus | null;
  error: string | null;
};

export type Overview = {
  bots: BotSnapshot[];
  health: { bot: string; segments: HealthSegment[] }[];
};

export type BotDetail = {
  latest: BotSnapshot;
  hours: number;
  segments: HealthSegment[];
  jobs: { ts: number; active: number | null; failed: number | null }[];
  panics: { ts: number; value: number | null }[];
  commands: { ts: number; value: number | null }[];
  dispatchErrors: { ts: number; value: number | null }[];
  /// Per-poll LLM token delta (prompt + completion).
  tokens: { ts: number; value: number | null }[];
  /// Cumulative LLM cost in USD.
  cost: { ts: number; value: number | null }[];
  commandBreakdown: { name: string; total: number; errors: number }[];
  restarts: { ts: number }[];
  deploys: { ts: number; from: string; to: string }[];
  errors: { ts: number; end: number; message: string }[];
};

export type MonitorStatus = {
  service: string;
  uptime_secs: number;
  bots_configured: number;
  last_poll_ago_secs: number | null;
  poll_errors_total: number;
  snapshots_total: number;
};

const DEFAULT_API = "http://127.0.0.1:9110";
const OVERVIEW_WINDOW_HOURS = 24;

function apiBase(): string {
  return process.env.MONITOR_API_URL ?? DEFAULT_API;
}

function isBotSnapshotArray(data: unknown): data is BotSnapshot[] {
  if (!Array.isArray(data)) return false;
  return data.every((item) => {
    if (typeof item !== "object" || item === null) return false;
    return (
      "bot" in item && typeof item.bot === "string" && "ts" in item && typeof item.ts === "number"
    );
  });
}

function isMonitorStatus(data: unknown): data is MonitorStatus {
  if (typeof data !== "object" || data === null) return false;
  return "service" in data && typeof data.service === "string";
}

async function apiBots(base: string): Promise<BotSnapshot[]> {
  const resp = await fetch(`${base}/api/bots`);
  if (!resp.ok) throw new Error(`monitor api responded ${resp.status}`);
  const data: unknown = await resp.json();
  if (!isBotSnapshotArray(data)) throw new Error("monitor api returned an unexpected shape");
  return data;
}

async function apiHistory(base: string, name: string, limit: number): Promise<BotSnapshot[]> {
  const resp = await fetch(`${base}/api/bots/${name}/history?limit=${limit}`);
  if (!resp.ok) throw new Error(`monitor api responded ${resp.status}`);
  const data: unknown = await resp.json();
  if (!isBotSnapshotArray(data)) throw new Error("monitor api returned an unexpected shape");
  return data;
}

/// Newest snapshot per bot.
export const fetchBots = createServerFn({ method: "GET" }).handler(
  async (): Promise<BotSnapshot[]> => {
    return apiBots(apiBase());
  },
);

/// Overview board: latest snapshots plus a 24h availability strip per bot.
export const fetchOverview = createServerFn({ method: "GET" }).handler(
  async (): Promise<Overview> => {
    const base = apiBase();
    const bots = await apiBots(base);
    const health = await Promise.all(
      bots.map(async (bot) => ({
        bot: bot.bot,
        segments: toSegments(await apiHistory(base, bot.bot, hoursToLimit(OVERVIEW_WINDOW_HOURS))),
      })),
    );
    return { bots, health };
  },
);

/// Everything the per-bot detail view renders, reduced server-side.
export const fetchBotDetail = createServerFn({ method: "POST" })
  .validator((d: { name: string; hours: number }) => d)
  .handler(async ({ data }): Promise<BotDetail> => {
    const base = apiBase();
    const history = await apiHistory(base, data.name, hoursToLimit(data.hours));
    const latest = history[0] ?? {
      bot: data.name,
      ts: Math.floor(Date.now() / 1000),
      status: null,
      error: "no data yet",
    };
    return buildBotDetail(data.hours, history, latest);
  });

/// The monitor's own runtime status.
export const fetchMonitorStatus = createServerFn({ method: "GET" }).handler(
  async (): Promise<MonitorStatus> => {
    const resp = await fetch(`${apiBase()}/metrics`);
    if (!resp.ok) throw new Error(`monitor metrics responded ${resp.status}`);
    const data: unknown = await resp.json();
    if (!isMonitorStatus(data)) throw new Error("monitor metrics returned an unexpected shape");
    return data;
  },
);
