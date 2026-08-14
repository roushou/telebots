import { Link } from "@tanstack/react-router";

import type { BotSnapshot } from "../lib/api";
import { healthOf, type Health } from "../lib/health";
import type { HealthSegment } from "../lib/history";
import { cn } from "../lib/utils";
import { AvailabilityBand } from "./charts";
import { Card, CardContent, CardHeader, CardTitle } from "./ui/card";

function dot(health: Health): string {
  if (health === "ok") return "bg-emerald-500";
  if (health === "degraded") return "bg-amber-500";
  return "bg-destructive";
}

export function HealthStrip({
  bots,
  health,
}: {
  bots: BotSnapshot[];
  health: { bot: string; segments: HealthSegment[] }[];
}) {
  const byBot = new Map(health.map((h) => [h.bot, h.segments]));

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base font-semibold">Last 24 hours</CardTitle>
      </CardHeader>
      <CardContent className="space-y-1">
        {bots.map((bot) => {
          const segments = byBot.get(bot.bot) ?? [];
          const botHealth = healthOf(bot);
          return (
            <Link
              key={bot.bot}
              to="/bots/$name"
              params={{ name: bot.bot }}
              className="flex items-center gap-3 rounded-md px-1 py-1.5 hover:bg-accent"
            >
              <span className="flex w-28 shrink-0 items-center gap-2 text-sm font-medium">
                <span
                  className={cn("size-2 shrink-0 rounded-full", dot(botHealth))}
                  aria-hidden="true"
                />
                <span className="truncate">{bot.bot}</span>
              </span>
              <div className="min-w-0 flex-1">
                <AvailabilityBand segments={segments} height={20} />
              </div>
            </Link>
          );
        })}
      </CardContent>
    </Card>
  );
}
