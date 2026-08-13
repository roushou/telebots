import { useNavigate } from "@tanstack/react-router";
import { Command } from "cmdk";
import { Bot, LayoutDashboard, Moon, Sun } from "lucide-react";
import { useEffect } from "react";

import type { BotSnapshot } from "../lib/api";
import { useTheme } from "../lib/theme";

const overlayClass =
  "fixed inset-0 z-50 bg-black/50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0";
const contentClass =
  "fixed left-1/2 top-[20%] z-50 w-full max-w-lg -translate-x-1/2 overflow-hidden rounded-lg border bg-popover text-popover-foreground shadow-lg";

export function CommandPalette({
  bots,
  open,
  onOpenChange,
}: {
  bots: BotSnapshot[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const { theme, toggleTheme } = useTheme();

  useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        onOpenChange(!open);
      }
    };
    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, [open, onOpenChange]);

  const go = (to: "/" | "/bots/$name", name?: string) => {
    onOpenChange(false);
    if (to === "/bots/$name" && name) {
      void navigate({ to, params: { name } });
    } else {
      void navigate({ to: "/" });
    }
  };

  return (
    <Command.Dialog
      open={open}
      onOpenChange={onOpenChange}
      label="Command menu"
      overlayClassName={overlayClass}
      contentClassName={contentClass}
    >
      <Command.Input
        className="flex h-12 w-full rounded-md bg-transparent px-4 text-sm outline-none placeholder:text-muted-foreground"
        placeholder="Search bots or run a command…"
      />
      <Command.List className="max-h-80 overflow-y-auto overflow-x-hidden border-t p-1">
        <Command.Empty className="py-6 text-center text-sm text-muted-foreground">
          No results found.
        </Command.Empty>

        <Command.Group
          className="overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground"
          heading="Navigate"
        >
          <Command.Item
            onSelect={() => go("/")}
            className="relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-2 text-sm outline-none data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground"
          >
            <LayoutDashboard className="size-4" />
            Overview
          </Command.Item>
        </Command.Group>

        <Command.Group
          className="overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground"
          heading="Bots"
        >
          {bots.map((bot) => (
            <Command.Item
              key={bot.bot}
              onSelect={() => go("/bots/$name", bot.bot)}
              className="relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-2 text-sm outline-none data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground"
            >
              <Bot className="size-4 text-muted-foreground" />
              {bot.bot}
            </Command.Item>
          ))}
        </Command.Group>

        <Command.Group
          className="overflow-hidden p-1 text-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground"
          heading="Preferences"
        >
          <Command.Item
            onSelect={() => toggleTheme()}
            className="relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-2 text-sm outline-none data-[selected=true]:bg-accent data-[selected=true]:text-accent-foreground"
          >
            {theme === "dark" ? <Sun className="size-4" /> : <Moon className="size-4" />}
            Toggle theme
          </Command.Item>
        </Command.Group>
      </Command.List>
    </Command.Dialog>
  );
}
