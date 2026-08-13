import { areaY, defineChart, lineY, rect } from "@tanstack/charts";
import { Chart } from "@tanstack/charts/react";
import { scaleLinear } from "@tanstack/charts/scales/linear";
import { tooltip } from "@tanstack/charts/tooltip";
import { scaleUtc } from "d3-scale";
import { useMemo } from "react";

import { fmtClock } from "../lib/format";
import type { HealthSegment } from "../lib/history";

type SegRow = { x1: Date; x2: Date };

function toSegRows(segments: HealthSegment[], up: boolean): SegRow[] {
  return segments
    .filter((s) => s.up === up)
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
    const upRows = toSegRows(segments, true);
    const downRows = toSegRows(segments, false);
    return defineChart({
      marks: [
        rect(upRows, {
          x1: "x1",
          x2: "x2",
          y1: () => 0,
          y2: () => 1,
          fill: "var(--chart-up)",
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

export function PanicsChart({
  points,
  height,
  className,
}: {
  points: { ts: number; value: number | null }[];
  height: number;
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
          fill: "var(--ts-chart-3)",
          fillOpacity: 0.16,
        }),
        lineY(rows, {
          x: "date",
          y: "value",
          stroke: "var(--ts-chart-3)",
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
      ariaLabel="Panics over time"
      className={className}
    />
  );
}
