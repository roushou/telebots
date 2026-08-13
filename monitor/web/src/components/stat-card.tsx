import type { ReactNode } from "react";

import { cn } from "../lib/utils";
import { Card, CardContent } from "./ui/card";

export function StatCard({
  label,
  value,
  hint,
  tone = "default",
}: {
  label: string;
  value: ReactNode;
  hint?: string;
  tone?: "default" | "destructive";
}) {
  return (
    <Card>
      <CardContent className="p-5">
        <div className="text-sm font-medium text-muted-foreground">{label}</div>
        <div
          className={cn(
            "mt-1 text-2xl font-semibold tabular-nums tracking-tight",
            tone === "destructive" && "text-destructive",
          )}
        >
          {value}
        </div>
        {hint ? <div className="mt-1 text-xs text-muted-foreground">{hint}</div> : null}
      </CardContent>
    </Card>
  );
}
