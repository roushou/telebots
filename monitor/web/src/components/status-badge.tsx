import type { Health } from "../lib/health";
import { cn } from "../lib/utils";
import { Badge } from "./ui/badge";

const TONES: Record<Health, { label: string; variant: "success" | "warning" | "destructive" }> = {
  ok: { label: "ok", variant: "success" },
  degraded: { label: "degraded", variant: "warning" },
  down: { label: "down", variant: "destructive" },
};

export function StatusBadge({ status, className }: { status: Health; className?: string }) {
  const tone = TONES[status];
  return (
    <Badge variant={tone.variant} className={cn("gap-1.5", className)}>
      <span className="size-1.5 rounded-full bg-current" aria-hidden="true" />
      {tone.label}
    </Badge>
  );
}
