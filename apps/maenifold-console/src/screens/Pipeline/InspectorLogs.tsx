import { useEffect, useMemo, useRef, useState } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import {
  collectNodeLogs,
  type LogEntry,
  type PipelineNode,
} from "@/lib/pipeline";

type InspectorLogsProps = {
  node: PipelineNode;
};

export function InspectorLogs({ node }: InspectorLogsProps) {
  const [levelFilter, setLevelFilter] = useState<"all" | "info" | "warn" | "error">("all");
  const [autoScroll, setAutoScroll] = useState(true);
  const parentRef = useRef<HTMLDivElement | null>(null);
  const entries = useMemo(() => collectNodeLogs(node), [node]);
  const filteredEntries = useMemo(
    () =>
      levelFilter === "all"
        ? entries
        : entries.filter((entry) => entry.level === levelFilter),
    [entries, levelFilter],
  );

  const columns = useMemo<ColumnDef<LogEntry>[]>(
    () => [
      {
        accessorKey: "timestamp",
        header: "Timestamp",
        cell: ({ row }) => (
          <span className="font-mono text-xs text-muted-foreground">
            {row.original.timestamp ?? "—"}
          </span>
        ),
        size: 220,
      },
      {
        accessorKey: "level",
        header: "Level",
        cell: ({ row }) => <LevelBadge level={row.original.level} />,
        size: 100,
      },
      {
        accessorKey: "message",
        header: "Message",
        cell: ({ row }) => (
          <div className="space-y-1">
            <div className="whitespace-pre-wrap break-words text-sm leading-relaxed">
              {row.original.message}
            </div>
            <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
              {row.original.source}
            </div>
          </div>
        ),
      },
    ],
    [],
  );

  const table = useReactTable({
    data: filteredEntries,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const rows = table.getRowModel().rows;
  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 72,
    overscan: 8,
  });

  useEffect(() => {
    if (!autoScroll || rows.length === 0) {
      return;
    }

    rowVirtualizer.scrollToIndex(rows.length - 1, { align: "end" });
  }, [autoScroll, rowVirtualizer, rows.length]);

  if (entries.length === 0) {
    return (
      <div className="flex h-full items-center justify-center rounded-lg border border-dashed border-border bg-muted/30 px-6 text-center text-sm text-muted-foreground">
        No logs available for this node yet.
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap gap-2">
          {(["all", "info", "warn", "error"] as const).map((level) => (
            <Button
              key={level}
              type="button"
              variant={levelFilter === level ? "secondary" : "ghost"}
              size="xs"
              onClick={() => setLevelFilter(level)}
            >
              {level.toUpperCase()}
            </Button>
          ))}
        </div>
        <label className="flex items-center gap-2 text-sm text-muted-foreground">
          <Switch checked={autoScroll} onCheckedChange={setAutoScroll} />
          Auto-scroll
        </label>
      </div>

      <div className="overflow-hidden rounded-lg border border-border bg-card/60">
        <table className="w-full border-collapse text-sm">
          <thead className="bg-muted/40">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <th
                    key={header.id}
                    className="px-3 py-2 text-left font-medium text-muted-foreground"
                    style={{
                      width: header.getSize() > 0 ? header.getSize() : undefined,
                    }}
                  >
                    {header.isPlaceholder
                      ? null
                      : flexRender(
                          header.column.columnDef.header,
                          header.getContext(),
                        )}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
        </table>
        <div ref={parentRef} className="h-[22rem] overflow-auto">
          <div
            className="relative w-full"
            style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
          >
            <table className="absolute left-0 top-0 w-full border-collapse text-sm">
              <tbody>
                {rowVirtualizer.getVirtualItems().map((virtualRow) => {
                  const row = rows[virtualRow.index];

                  return (
                    <tr
                      key={row.id}
                      className="absolute left-0 top-0 w-full border-t border-border/70"
                      style={{
                        transform: `translateY(${virtualRow.start}px)`,
                      }}
                    >
                      {row.getVisibleCells().map((cell) => (
                        <td
                          key={cell.id}
                          className="px-3 py-3 align-top"
                          style={{
                            width:
                              cell.column.getSize() > 0
                                ? cell.column.getSize()
                                : undefined,
                          }}
                        >
                          {flexRender(
                            cell.column.columnDef.cell,
                            cell.getContext(),
                          )}
                        </td>
                      ))}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

function LevelBadge({ level }: { level: LogEntry["level"] }) {
  const className =
    level === "error"
      ? "border-destructive/30 bg-destructive/10 text-destructive"
      : level === "warn"
        ? "border-chart-3/30 bg-chart-3/10 text-chart-3"
        : "border-chart-5/30 bg-chart-5/10 text-chart-5";

  return (
    <Badge variant="outline" className={className}>
      {level.toUpperCase()}
    </Badge>
  );
}
