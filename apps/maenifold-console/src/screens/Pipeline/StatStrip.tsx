import { useMemo } from "react";
import {
  Line,
  LineChart,
  ResponsiveContainer,
} from "recharts";

import { Card, CardContent } from "@/components/ui/card";
import {
  formatDuration,
  type RunMetadata,
} from "@/lib/pipeline";

type StatStripProps = {
  metadata: RunMetadata;
  activeHistory: number[];
  completedHistory: number[];
  failedHistory: number[];
  elapsedHistory: number[];
};

export function StatStrip({
  metadata,
  activeHistory,
  completedHistory,
  failedHistory,
  elapsedHistory,
}: StatStripProps) {
  const cards = useMemo(
    () => [
      {
        label: "ACTIVE NODES",
        value: String(metadata.activeNodes),
        history: activeHistory,
        color: "var(--color-chart-5)",
      },
      {
        label: "COMPLETED",
        value: String(metadata.completedNodes),
        history: completedHistory,
        color: "var(--color-chart-2)",
      },
      {
        label: "FAILED",
        value: String(metadata.failedNodes),
        history: failedHistory,
        color: "var(--color-destructive)",
      },
      {
        label: "ELAPSED",
        value: formatDuration(metadata.elapsedMs),
        history: elapsedHistory,
        color: "var(--color-primary)",
        pulse: metadata.hasActiveRun,
      },
    ],
    [activeHistory, completedHistory, elapsedHistory, failedHistory, metadata],
  );

  return (
    <div className="grid gap-3 md:grid-cols-4">
      {cards.map((card) => (
        <Card
          key={card.label}
          className={card.pulse ? "shadow-[0_0_0_1px_var(--color-primary)]" : undefined}
        >
          <CardContent className="flex h-14 items-center justify-between gap-4 px-4 py-2">
            <div className="space-y-1">
              <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
                {card.label}
              </div>
              <div className="text-lg font-semibold tracking-tight">{card.value}</div>
            </div>
            <div className="h-9 w-24">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart
                  data={card.history.map((value, index) => ({ index, value }))}
                  margin={{ top: 4, right: 0, left: 0, bottom: 0 }}
                >
                  <Line
                    type="monotone"
                    dataKey="value"
                    stroke={card.color}
                    strokeWidth={2}
                    dot={false}
                    isAnimationActive={false}
                  />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
