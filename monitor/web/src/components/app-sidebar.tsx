import { Link } from "@tanstack/react-router";
import { Activity, Bot, LayoutDashboard } from "lucide-react";
import { cn } from "../lib/utils";
import type { BotSnapshot } from "../lib/api";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";

function isOk(bot: BotSnapshot): boolean {
  return bot.status !== null && bot.error === null;
}

function Brand({ collapsed }: { collapsed: boolean }) {
  return (
    <Link
      to="/"
      className="mb-2 flex h-10 items-center gap-2.5 rounded-md px-2"
    >
      <span className="flex size-7 shrink-0 items-center justify-center rounded-md bg-primary text-primary-foreground">
        <Activity className="size-4" />
      </span>
      {!collapsed && <span className="text-sm font-semibold">Telebots</span>}
    </Link>
  );
}

export function AppSidebar({
  bots,
  collapsed,
  onNavigate,
  className,
}: {
  bots: BotSnapshot[];
  collapsed: boolean;
  onNavigate?: () => void;
  className?: string;
}) {
  return (
    <nav className={cn("flex h-full flex-col gap-1 px-3 py-4", className)}>
      <Brand collapsed={collapsed} />

      <Link
        to="/"
        onClick={onNavigate}
        activeOptions={{ exact: true }}
        activeProps={{ className: "bg-accent text-accent-foreground" }}
        className="flex h-9 items-center gap-2.5 rounded-md px-2 text-sm font-medium text-muted-foreground hover:bg-accent hover:text-accent-foreground"
      >
        <LayoutDashboard className="size-4 shrink-0" />
        {!collapsed && <span>Overview</span>}
      </Link>

      {!collapsed && (
        <div className="mt-4 px-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
          Bots
        </div>
      )}

      <div className="flex flex-col gap-0.5">
        {bots.map((bot) => {
          const ok = isOk(bot);
          const link = (
            <Link
              key={bot.bot}
              to="/bots/$name"
              params={{ name: bot.bot }}
              onClick={onNavigate}
              activeProps={{ className: "bg-accent text-accent-foreground" }}
              className="flex h-9 items-center gap-2.5 rounded-md px-2 text-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground"
            >
              <Bot
                className={cn(
                  "size-4 shrink-0",
                  collapsed && (ok ? "text-emerald-500" : "text-destructive")
                )}
              />
              {!collapsed && <span className="flex-1 truncate">{bot.bot}</span>}
              {!collapsed && (
                <span
                  className={cn(
                    "size-2 shrink-0 rounded-full",
                    ok ? "bg-emerald-500" : "bg-destructive"
                  )}
                  aria-hidden="true"
                />
              )}
            </Link>
          );
          return collapsed ? (
            <Tooltip key={bot.bot}>
              <TooltipTrigger asChild>{link}</TooltipTrigger>
              <TooltipContent side="right">{bot.bot}</TooltipContent>
            </Tooltip>
          ) : (
            link
          );
        })}
      </div>
    </nav>
  );
}
