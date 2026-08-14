import { createFileRoute, Link, useNavigate } from "@tanstack/react-router";
import { ArrowLeft } from "lucide-react";
import { useEffect, useState } from "react";

import { AvailabilityBand, CounterChart, JobsChart } from "../../../components/charts";
import { StatCard } from "../../../components/stat-card";
import { StatusBadge } from "../../../components/status-badge";
import { Button } from "../../../components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "../../../components/ui/card";
import { fetchBotDetail, type BotDetail } from "../../../lib/api";
import { fmtDuration, fmtPct, fmtStamp, fmtUptime } from "../../../lib/format";
import { healthOf } from "../../../lib/health";
import { summarizeHealth } from "../../../lib/history";

const RANGES = [1, 6, 24] as const;

type Search = { hours?: number };

export const Route = createFileRoute("/_app/bots/$name")({
  validateSearch: (search: Record<string, unknown>): Search => {
    const h = Number(search.hours);
    return { hours: h === 1 || h === 6 || h === 24 ? h : undefined };
  },
  loaderDeps: ({ search }) => ({ hours: search.hours ?? 24 }),
  loader: async ({ params, deps }): Promise<BotDetail> => {
    try {
      return await fetchBotDetail({
        data: { name: params.name, hours: deps.hours },
      });
    } catch {
      return emptyDetail(params.name, deps.hours);
    }
  },
  component: BotDetailPage,
});

function emptyDetail(name: string, hours: number): BotDetail {
  return {
    latest: { bot: name, ts: 0, status: null, error: "no data yet" },
    hours,
    segments: [],
    jobs: [],
    panics: [],
    commands: [],
    dispatchErrors: [],
    restarts: [],
    deploys: [],
    errors: [],
  };
}

function BotDetailPage() {
  const { name } = Route.useParams();
  const { hours } = Route.useSearch();
  const range = hours ?? 24;
  const navigate = useNavigate();
  const initial = Route.useLoaderData();
  const [data, setData] = useState<BotDetail>(initial);

  useEffect(() => {
    let cancelled = false;
    async function refresh() {
      try {
        const next = await fetchBotDetail({ data: { name, hours: range } });
        if (!cancelled) setData(next);
      } catch {
        // keep previous state
      }
    }
    const id = setInterval(refresh, 15_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [name, range]);

  const { latest } = data;
  const status = latest.status;
  const health = healthOf(latest);
  const summary = summarizeHealth(data.segments);

  return (
    <div className="space-y-6">
      <div>
        <Link
          to="/"
          className="mb-3 inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="size-4" />
          Overview
        </Link>
        <div className="flex flex-wrap items-center gap-3">
          <h1 className="text-2xl font-semibold tracking-tight">{name}</h1>
          <StatusBadge status={health} />
        </div>
        <p className="mt-1 text-sm text-muted-foreground">
          {status
            ? `v${status.version} · telegram ${status.telegram === "ok" ? "reachable" : "unreachable"}`
            : (latest.error ?? "no data yet")}
        </p>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="inline-flex rounded-md border p-0.5">
          {RANGES.map((h) => (
            <Button
              key={h}
              variant={range === h ? "secondary" : "ghost"}
              size="sm"
              onClick={() =>
                navigate({
                  to: "/bots/$name",
                  params: { name },
                  search: { hours: h },
                })
              }
            >
              {h}h
            </Button>
          ))}
        </div>
        <span className="text-xs text-muted-foreground">history stored at 30s intervals</span>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        <StatCard
          label="Availability"
          value={fmtPct(summary.uptimePct)}
          hint={`${fmtDuration(summary.downtimeSecs)} down · longest ${fmtDuration(summary.longestOutageSecs)}`}
        />
        <StatCard label="Uptime" value={status ? fmtUptime(status.uptime_secs) : "—"} />
        <StatCard label="Commands" value={status?.commands_total ?? "—"} />
        <StatCard label="Jobs active" value={status?.jobs_active ?? "—"} />
        <StatCard
          label="Jobs failed"
          value={status?.jobs_failed_total ?? "—"}
          tone={status && status.jobs_failed_total > 0 ? "destructive" : "default"}
        />
        <StatCard
          label="Dispatch errors"
          value={status?.dispatch_errors_total ?? "—"}
          tone={status && status.dispatch_errors_total > 0 ? "destructive" : "default"}
        />
        <StatCard
          label="Panics"
          value={status?.panics_total ?? "—"}
          tone={status && status.panics_total > 0 ? "destructive" : "default"}
        />
      </div>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base font-semibold">Availability</CardTitle>
        </CardHeader>
        <CardContent>
          <AvailabilityBand segments={data.segments} height={160} showAxis />
        </CardContent>
      </Card>

      <div className="grid gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Jobs</CardTitle>
          </CardHeader>
          <CardContent>
            <JobsChart points={data.jobs} height={200} />
            <p className="mt-2 text-xs text-muted-foreground">
              Blue: active · violet: failed total
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Panics</CardTitle>
          </CardHeader>
          <CardContent>
            <CounterChart
              points={data.panics}
              color="var(--ts-chart-3)"
              height={200}
              ariaLabel="Panics over time"
            />
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Commands</CardTitle>
          </CardHeader>
          <CardContent>
            <CounterChart
              points={data.commands}
              color="var(--ts-chart-1)"
              height={200}
              ariaLabel="Commands per poll over time"
            />
            <p className="mt-2 text-xs text-muted-foreground">commands per 30s poll</p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Dispatch errors</CardTitle>
          </CardHeader>
          <CardContent>
            <CounterChart
              points={data.dispatchErrors}
              color="var(--ts-chart-5)"
              height={200}
              ariaLabel="Dispatch errors per poll over time"
            />
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 lg:grid-cols-3">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Restarts</CardTitle>
          </CardHeader>
          <CardContent>
            {data.restarts.length === 0 ? (
              <p className="text-sm text-muted-foreground">No restarts detected in this window.</p>
            ) : (
              <ul className="space-y-2">
                {data.restarts.map((r) => (
                  <li key={r.ts} className="flex items-center gap-2 text-sm">
                    <span className="size-1.5 rounded-full bg-amber-500" aria-hidden="true" />
                    <span className="tabular-nums">{fmtStamp(r.ts)}</span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Deploys</CardTitle>
          </CardHeader>
          <CardContent>
            {data.deploys.length === 0 ? (
              <p className="text-sm text-muted-foreground">No deploys detected in this window.</p>
            ) : (
              <ul className="space-y-2">
                {data.deploys.map((d) => (
                  <li key={d.ts} className="flex items-center gap-2 text-sm">
                    <span className="size-1.5 rounded-full bg-sky-500" aria-hidden="true" />
                    <span className="tabular-nums">{fmtStamp(d.ts)}</span>
                    <span className="text-muted-foreground">
                      {d.from} → {d.to}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base font-semibold">Errors</CardTitle>
          </CardHeader>
          <CardContent>
            {data.errors.length === 0 ? (
              <p className="text-sm text-muted-foreground">No errors in this window.</p>
            ) : (
              <ul className="space-y-2">
                {data.errors.map((e) => (
                  <li key={e.ts} className="text-sm">
                    <span className="text-xs tabular-nums text-muted-foreground">
                      {fmtStamp(e.ts)} → {fmtStamp(e.end)} · {fmtDuration(e.end - e.ts)}
                    </span>
                    <p className="font-mono text-xs text-muted-foreground">{e.message}</p>
                  </li>
                ))}
              </ul>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
