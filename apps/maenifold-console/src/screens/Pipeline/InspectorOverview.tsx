import { useMemo } from "react";
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
} from "@tanstack/react-table";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import { ScrollArea } from "@/components/ui/scroll-area";
import { type PipelineNode, type PipelineView } from "@/lib/pipeline";

type OverviewRow = {
  key: string;
  value: string;
};

type InspectorOverviewProps = {
  pipeline: PipelineView;
  node: PipelineNode;
};

export function InspectorOverview({
  pipeline,
  node,
}: InspectorOverviewProps) {
  const rows = useMemo<OverviewRow[]>(
    () => [
      { key: "Pipeline", value: pipeline.name ?? pipeline.pipelineId },
      { key: "Node", value: node.nodeId },
      { key: "Type", value: node.type },
      { key: "Status", value: node.status },
      { key: "Session", value: node.sessionId },
      { key: "Agent", value: node.agent ?? node.acpClient ?? "—" },
      { key: "Workflow", value: node.workflowId ?? node.toolName ?? node.topic ?? "—" },
      { key: "Attempts", value: String(node.attempts) },
      { key: "Started", value: node.startedAt ?? "—" },
      { key: "Completed", value: node.completedAt ?? "—" },
      { key: "Input tokens", value: node.inputTokens ? String(node.inputTokens) : "—" },
      { key: "Output tokens", value: node.outputTokens ? String(node.outputTokens) : "—" },
    ],
    [node, pipeline.name, pipeline.pipelineId],
  );

  const columns = useMemo<ColumnDef<OverviewRow>[]>(
    () => [
      {
        accessorKey: "key",
        header: "Field",
        cell: ({ row }) => (
          <span className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
            {row.original.key}
          </span>
        ),
      },
      {
        accessorKey: "value",
        header: "Value",
      },
    ],
    [],
  );

  const table = useReactTable({
    data: rows,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const markdown =
    pipeline.description ??
    node.assembled_prompt ??
    node.prompt ??
    "No description available for this node.";

  return (
    <ScrollArea className="h-full">
      <div className="space-y-4 pr-4">
        <div className="overflow-hidden rounded-lg border border-border bg-card/60">
          <table className="w-full border-collapse text-sm">
            <thead className="bg-muted/40">
              {table.getHeaderGroups().map((headerGroup) => (
                <tr key={headerGroup.id}>
                  {headerGroup.headers.map((header) => (
                    <th
                      key={header.id}
                      className="px-3 py-2 text-left font-medium text-muted-foreground"
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
            <tbody>
              {table.getRowModel().rows.map((row) => (
                <tr key={row.id} className="border-t border-border/70">
                  {row.getVisibleCells().map((cell) => (
                    <td key={cell.id} className="px-3 py-2 align-top">
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="rounded-lg border border-border bg-card/60 p-4">
          <div className="mb-3 font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
            Description
          </div>
          <div className="pipeline-markdown text-sm">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
