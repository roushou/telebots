import { areaY, defineChart, lineY, rect } from "@tanstack/charts";
import { Chart } from "@tanstack/charts/react";
import { scaleLinear } from "@tanstack/charts/scales/linear";
import { tooltip } from "@tanstack/charts/tooltip";
import { scaleUtc } from "d3-scale";
import { useMemo } from "react";

import { fmtClock } from "../lib/format";
import type { Health } from "../lib/health";
import type { HealthSegment } from "../lib/history";

type SegRow = { x1: Date; x2: Date };

function toSegRows(segments: HealthSegment[], status: Health): SegRow[] {
  return segments
    .filter((s) => s.status === status)
    .map((s) => ({
      x1: new Date(s.start * 1000),
      x2: new Date(s.end * 1000),
    }));
}

const tickFormat = (d: Date) => fmtClock(d.getTime());

/// Up/down band over time. Compact (axis-less) in the health strip, full
/// time axis on the detail view.
export function AvailabilityBand({
  segments,
  height,
  showAxis = false,
  className,
}: {
  segments: HealthSegment[];
  height: number;
  showAxis?: boolean;
  className?: string;
}) {
  const definition = useMemo(() => {
    const okRows = toSegRows(segments, "ok");
    const degradedRows = toSegRows(segments, "degraded");
    const downRows = toSegRows(segments, "down");
    return defineChart({
      marks: [
        rect(okRows, {
          x1: "x1",
          x2: "x2",
          y1: () => 0,
          y2: () => 1,
          fill: "var(--chart-up)",
        }),
        rect(degradedRows, {
          x1: "x1",
          x2: "x2",
          y1: () => 0,
          y2: () => 1,
          fill: "var(--chart-degraded)",
        }),
        rect(downRows, {
          x1: "x1",
          x2: "x2",
          y1: () => 0,
          y2: () => 1,
          fill: "var(--chart-down)",
        }),
      ],
      x: {
        scale: scaleUtc,
        axis: showAxis ? { ticks: { count: 5, format: tickFormat } } : false,
      },
      y: { scale: () => scaleLinear().domain([0, 1]), axis: false, grid: false },
      ...(showAxis ? { tooltip } : { guides: false, margin: 0 }),
    });
  }, [segments, showAxis]);

  return (
    <Chart
      definition={definition}
      height={height}
      initialWidth={640}
      ariaLabel="Availability over time"
      className={className}
    />
  );
}

export type JobPoint = { ts: number; active: number | null; failed: number | null };

export function JobsChart({
  points,
  height,
  className,
}: {
  points: JobPoint[];
  height: number;
  className?: string;
}) {
  const definition = useMemo(() => {
    const rows = points.map((p) => ({
      date: new Date(p.ts * 1000),
      active: p.active,
      failed: p.failed,
    }));
    return defineChart({
      marks: [
        lineY(rows, {
          x: "date",
          y: "active",
          stroke: "var(--ts-chart-1)",
          strokeWidth: 2,
        }),
        lineY(rows, {
          x: "date",
          y: "failed",
          stroke: "var(--ts-chart-2)",
          strokeWidth: 2,
        }),
      ],
      x: { scale: scaleUtc, axis: { ticks: { count: 5, format: tickFormat } } },
      y: { scale: scaleLinear, nice: true, grid: true },
      tooltip,
    });
  }, [points]);

  return (
    <Chart
      definition={definition}
      height={height}
      initialWidth={640}
      ariaLabel="Active and failed jobs over time"
      className={className}
    />
  );
}

/// A single-series area/line chart for a counter (panics, commands, ...).
export function CounterChart({
  points,
  color,
  height,
  ariaLabel,
  className,
}: {
  points: { ts: number; value: number | null }[];
  color: string;
  height: number;
  ariaLabel: string;
  className?: string;
}) {
  const definition = useMemo(() => {
    const rows = points.map((p) => ({
      date: new Date(p.ts * 1000),
      value: p.value,
    }));
    return defineChart({
      marks: [
        areaY(rows, {
          x: "date",
          y: "value",
          fill: color,
          fillOpacity: 0.16,
        }),
        lineY(rows, {
          x: "date",
          y: "value",
          stroke: color,
          strokeWidth: 2,
        }),
      ],
      x: { scale: scaleUtc, axis: { ticks: { count: 5, format: tickFormat } } },
      y: { scale: scaleLinear, nice: true, grid: true },
      tooltip,
    });
  }, [points, color]);

  return (
    <Chart
      definition={definition}
      height={height}
      initialWidth={640}
      ariaLabel={ariaLabel}
      className={className}
    />
  );
}
