import { Outlet, createFileRoute } from "@tanstack/react-router";
import { Menu, PanelLeftClose, PanelLeftOpen, Search } from "lucide-react";
import { useEffect, useState } from "react";

import { AppSidebar } from "../../components/app-sidebar";
import { CommandPalette } from "../../components/command-palette";
import { ThemeToggle } from "../../components/theme-toggle";
import { Button } from "../../components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "../../components/ui/sheet";
import { Tooltip, TooltipContent, TooltipTrigger } from "../../components/ui/tooltip";
import { fetchBots, type BotSnapshot } from "../../lib/api";
import { fmtAgo } from "../../lib/format";
import { cn } from "../../lib/utils";

export const Route = createFileRoute("/_app")({
  component: Layout,
  loader: async () => {
    try {
      return await fetchBots();
    } catch {
      return [];
    }
  },
});

function Layout() {
  const initial = Route.useLoaderData();
  const [bots, setBots] = useState<BotSnapshot[]>(initial);
  const [lastPoll, setLastPoll] = useState<number | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function refresh() {
      try {
        const data = await fetchBots();
        if (!cancelled) {
          setBots(data);
          setLastPoll(Date.now());
        }
      } catch {
        // keep the previous state; the monitor may be restarting
      }
    }
    void refresh();
    const id = setInterval(refresh, 15_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div className="flex min-h-screen">
      {/* Desktop navigation */}
      <aside
        className={cn(
          "sticky top-0 hidden h-screen shrink-0 flex-col border-r bg-muted/30 transition-[width] md:flex",
          collapsed ? "w-16" : "w-60",
        )}
      >
        <AppSidebar bots={bots} collapsed={collapsed} />
      </aside>

      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-30 flex h-14 items-center gap-1.5 border-b bg-background/80 px-3 backdrop-blur">
          {/* Mobile navigation */}
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon" className="md:hidden">
                <Menu className="size-5" />
                <span className="sr-only">Open navigation</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-64 p-0">
              <SheetHeader className="sr-only">
                <SheetTitle>Navigation</SheetTitle>
              </SheetHeader>
              <AppSidebar bots={bots} collapsed={false} />
            </SheetContent>
          </Sheet>

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="hidden md:inline-flex"
                onClick={() => setCollapsed((c) => !c)}
                aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
              >
                {collapsed ? (
                  <PanelLeftOpen className="size-5" />
                ) : (
                  <PanelLeftClose className="size-5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right">{collapsed ? "Expand" : "Collapse"}</TooltipContent>
          </Tooltip>

          <div className="flex-1" />

          {lastPoll && (
            <span className="hidden text-xs text-muted-foreground sm:inline">
              updated {fmtAgo(Math.round((Date.now() - lastPoll) / 1000))}
            </span>
          )}

          <Button
            variant="outline"
            className="h-9 gap-2 px-2.5 text-muted-foreground sm:px-3"
            onClick={() => setPaletteOpen(true)}
          >
            <Search className="size-4" />
            <span className="hidden sm:inline">Search</span>
            <kbd className="pointer-events-none hidden select-none rounded border bg-muted px-1.5 font-mono text-[10px] text-muted-foreground sm:inline">
              ⌘K
            </kbd>
          </Button>

          <ThemeToggle />
        </header>

        <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6 sm:px-6">
          <Outlet />
        </main>
      </div>

      <CommandPalette bots={bots} open={paletteOpen} onOpenChange={setPaletteOpen} />
    </div>
  );
}
