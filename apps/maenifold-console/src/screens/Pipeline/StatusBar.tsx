import {
  formatDuration,
  type RunMetadata,
} from "@/lib/pipeline";
import { type Transport } from "@/lib/bridge/transport";

type StatusBarProps = {
  metadata: RunMetadata;
  transport: Transport;
  isHealthy: boolean;
  uptimeMs: number;
  lastHeartbeatAt: number | null;
};

export function StatusBar({
  metadata,
  transport,
  isHealthy,
  uptimeMs,
  lastHeartbeatAt,
}: StatusBarProps) {
  const heartbeatLabel = lastHeartbeatAt
    ? new Date(lastHeartbeatAt).toLocaleTimeString()
    : "WAITING";

  return (
    <div className="flex h-7 flex-wrap items-center justify-between gap-3 border-t border-border px-4 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
      <div className="flex items-center gap-2">
        <span
          className={[
            "inline-flex size-2 rounded-full",
            isHealthy ? "bg-chart-2" : "bg-destructive",
          ].join(" ")}
        />
        <span>{isHealthy ? "Daemon healthy" : "Daemon degraded"}</span>
      </div>
      <div className="flex items-center gap-4">
        <span>Uptime {formatDuration(uptimeMs)}</span>
        <span>WS {transport.toUpperCase()}</span>
        <span>Heartbeat {heartbeatLabel}</span>
        <span>Run {formatDuration(metadata.elapsedMs)}</span>
      </div>
    </div>
  );
}
