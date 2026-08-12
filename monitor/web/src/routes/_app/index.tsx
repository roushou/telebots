import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { LayoutGrid, Table2 } from "lucide-react";
import { BotCard } from "../../components/bot-card";
import { BotTable } from "../../components/bot-table";
import { HealthStrip } from "../../components/health-strip";
import { StatCard } from "../../components/stat-card";
import { Button } from "../../components/ui/button";
import { fetchOverview, type Overview } from "../../lib/api";
import { fmtStamp } from "../../lib/format";

export const Route = createFileRoute("/_app/")({
  component: OverviewPage,
  loader: async (): Promise<Overview> => {
    try {
      return await fetchOverview();
    } catch {
      return { bots: [], health: [] };
    }
  },
});

function OverviewPage() {
  const initial = Route.useLoaderData();
  const [data, setData] = useState<Overview>(initial);
  const [view, setView] = useState<"grid" | "table">("grid");

  useEffect(() => {
    let cancelled = false;
    async function refresh() {
      try {
        const next = await fetchOverview();
        if (!cancelled) setData(next);
      } catch {
        // keep the previous state
      }
    }
    const id = setInterval(refresh, 15_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const segmentsFor = (bot: string) =>
    data.health.find((h) => h.bot === bot)?.segments ?? [];

  const up = data.bots.filter((b) => b.status !== null && b.error === null).length;
  const down = data.bots.length - up;
  const latestTs = data.bots.reduce((max, b) => Math.max(max, b.ts), 0);

  return (
    <div className="space-y-6">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Overview</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {data.bots.length} bot(s) monitored · polled every 30s
          </p>
        </div>

        <div className="flex rounded-md border p-0.5">
          <Button
            variant={view === "grid" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setView("grid")}
            className="gap-1.5"
          >
            <LayoutGrid className="size-4" />
            Grid
          </Button>
          <Button
            variant={view === "table" ? "secondary" : "ghost"}
            size="sm"
            onClick={() => setView("table")}
            className="gap-1.5"
          >
            <Table2 className="size-4" />
            Table
          </Button>
        </div>
      </header>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label="Bots" value={data.bots.length} hint="configured targets" />
        <StatCard label="Up" value={up} hint="healthy right now" />
        <StatCard
          label="Down"
          value={down}
          tone={down > 0 ? "destructive" : "default"}
          hint={down > 0 ? "needs attention" : "all clear"}
        />
        <StatCard
          label="Last snapshot"
          value={latestTs ? fmtStamp(latestTs) : "—"}
          hint="newest across bots"
        />
      </div>

      <HealthStrip bots={data.bots} health={data.health} />

      {data.bots.length === 0 ? (
        <div className="rounded-lg border border-dashed p-10 text-center text-sm text-muted-foreground">
          No bots to show. Configure <code className="font-mono">MONITOR_BOTS</code>{" "}
          and the monitor will start recording snapshots.
        </div>
      ) : view === "grid" ? (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {data.bots.map((bot) => (
            <BotCard key={bot.bot} bot={bot} segments={segmentsFor(bot.bot)} />
          ))}
        </div>
      ) : (
        <BotTable bots={data.bots} health={data.health} />
      )}
    </div>
  );
}
