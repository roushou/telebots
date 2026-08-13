import { cn } from "../lib/utils";
import { Badge } from "./ui/badge";

export function StatusBadge({ ok, className }: { ok: boolean; className?: string }) {
  return (
    <Badge variant={ok ? "success" : "destructive"} className={cn("gap-1.5", className)}>
      <span className="size-1.5 rounded-full bg-current" aria-hidden="true" />
      {ok ? "ok" : "down"}
    </Badge>
  );
}
