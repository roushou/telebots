import { Link } from "@tanstack/react-router";

import type { BotSnapshot } from "../lib/api";
import { fmtAgo, fmtUptime } from "../lib/format";
import type { HealthSegment } from "../lib/history";
import { cn } from "../lib/utils";
import { AvailabilityBand } from "./charts";
import { StatusBadge } from "./status-badge";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4">
      <span className="text-muted-foreground">{label}</span>
      <span className="tabular-nums">{value}</span>
    </div>
  );
}

export function BotCard({ bot, segments }: { bot: BotSnapshot; segments: HealthSegment[] }) {
  const { status, error } = bot;
  const ok = status !== null && error === null;

  return (
    <Card className="group transition-shadow hover:shadow-md">
      <Link to="/bots/$name" params={{ name: bot.bot }} className="block">
        <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-3">
          <CardTitle className="text-base font-semibold">{bot.bot}</CardTitle>
          <StatusBadge ok={ok} />
        </CardHeader>
        <CardContent className="space-y-3">
          <AvailabilityBand segments={segments} height={28} />

          {ok && status ? (
            <div className="space-y-1 text-sm">
              <Row label="version" value={status.version} />
              <Row label="uptime" value={fmtUptime(status.uptime_secs)} />
              <Row label="last command" value={fmtAgo(status.last_command_ago_secs)} />
              <Row label="commands" value={String(status.commands_total)} />
              <Row
                label="jobs"
                value={`${status.jobs_active} active · ${status.jobs_failed_total} failed`}
              />
              <Row label="panics" value={String(status.panics_total)} />
            </div>
          ) : (
            <p className={cn("text-sm text-muted-foreground")}>
              unreachable: {error ?? "no data yet"}
            </p>
          )}
        </CardContent>
      </Link>
    </Card>
  );
}
