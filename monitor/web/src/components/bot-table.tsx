import { Link } from "@tanstack/react-router";
import { Search } from "lucide-react";
import { useMemo, useState } from "react";

import type { BotSnapshot } from "../lib/api";
import { fmtAgo, fmtUptime } from "../lib/format";
import type { HealthSegment } from "../lib/history";
import { AvailabilityBand } from "./charts";
import { StatusBadge } from "./status-badge";
import { Input } from "./ui/input";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./ui/table";

export function BotTable({
  bots,
  health,
}: {
  bots: BotSnapshot[];
  health: { bot: string; segments: HealthSegment[] }[];
}) {
  const [query, setQuery] = useState("");
  const byBot = new Map(health.map((h) => [h.bot, h.segments]));

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return bots;
    return bots.filter((b) => b.bot.toLowerCase().includes(q));
  }, [bots, query]);

  return (
    <div className="space-y-3">
      <div className="relative max-w-xs">
        <Search className="absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Filter bots…"
          className="pl-8"
        />
      </div>

      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Bot</TableHead>
            <TableHead>Status</TableHead>
            <TableHead>Version</TableHead>
            <TableHead>Uptime</TableHead>
            <TableHead>Last command</TableHead>
            <TableHead>Commands</TableHead>
            <TableHead>Jobs (active / failed)</TableHead>
            <TableHead>Panics</TableHead>
            <TableHead className="w-40">24h</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {filtered.map((bot) => {
            const ok = bot.status !== null && bot.error === null;
            const status = bot.status;
            return (
              <TableRow key={bot.bot}>
                <TableCell className="font-medium">
                  <Link to="/bots/$name" params={{ name: bot.bot }} className="hover:underline">
                    {bot.bot}
                  </Link>
                </TableCell>
                <TableCell>
                  <StatusBadge ok={ok} />
                </TableCell>
                <TableCell className="text-muted-foreground">{status?.version ?? "—"}</TableCell>
                <TableCell className="tabular-nums">
                  {status ? fmtUptime(status.uptime_secs) : "—"}
                </TableCell>
                <TableCell className="tabular-nums">
                  {status ? fmtAgo(status.last_command_ago_secs) : "—"}
                </TableCell>
                <TableCell className="tabular-nums">{status?.commands_total ?? "—"}</TableCell>
                <TableCell className="tabular-nums">
                  {status ? `${status.jobs_active} / ${status.jobs_failed_total}` : "—"}
                </TableCell>
                <TableCell className="tabular-nums">{status?.panics_total ?? "—"}</TableCell>
                <TableCell>
                  <AvailabilityBand segments={byBot.get(bot.bot) ?? []} height={20} />
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
