import { useMemo } from "react";
import {
  Braces,
  Copy,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { stringifyJson, type JsonValue } from "@/lib/pipeline";

type PropertyTreeProps = {
  data: JsonValue | undefined;
  emptyMessage: string;
};

type PropertyNodeProps = {
  label: string;
  path: string;
  value: JsonValue;
};

export function PropertyTree({ data, emptyMessage }: PropertyTreeProps) {
  const hasData = data !== undefined && data !== null && stringifyJson(data) !== "";

  if (!hasData) {
    return (
      <div className="flex h-full items-center justify-center rounded-lg border border-dashed border-border bg-muted/30 px-6 text-center text-sm text-muted-foreground">
        {emptyMessage}
      </div>
    );
  }

  return (
    <ScrollArea className="h-full rounded-lg border border-border bg-background/60">
      <div className="space-y-3 p-4">
        <PropertyNode label="root" path="root" value={data as JsonValue} />
      </div>
    </ScrollArea>
  );
}

function PropertyNode({ label, path, value }: PropertyNodeProps) {
  const isBranch =
    typeof value === "object" && value !== null && Object.keys(value).length > 0;

  if (!isBranch) {
    return <PropertyLeaf label={label} path={path} value={value} />;
  }

  const entries = Array.isArray(value)
    ? value.map((item, index) => [String(index), item] as const)
    : Object.entries(value);

  return (
    <details open className="group rounded-lg border border-border bg-card/60">
      <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-2">
        <div className="flex items-center gap-2 text-sm">
          <Braces className="size-4 text-muted-foreground" />
          <span className="font-medium">{label}</span>
          <span className="font-mono text-xs uppercase tracking-[0.18em] text-muted-foreground">
            {Array.isArray(value) ? `ARRAY ${entries.length}` : `OBJECT ${entries.length}`}
          </span>
        </div>
      </summary>
      <Separator />
      <div className="space-y-2 p-3">
        {entries.map(([entryLabel, entryValue]) => (
          <PropertyNode
            key={`${path}.${entryLabel}`}
            label={entryLabel}
            path={`${path}.${entryLabel}`}
            value={entryValue}
          />
        ))}
      </div>
    </details>
  );
}

function PropertyLeaf({ label, path, value }: PropertyNodeProps) {
  const displayValue = useMemo(() => stringifyJson(value), [value]);

  return (
    <div className="rounded-lg border border-border/70 bg-background/70 px-3 py-2">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 space-y-1">
          <div className="text-sm font-medium">{label}</div>
          <div className="font-mono text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
            {path}
          </div>
          <div className="whitespace-pre-wrap break-words font-mono text-xs text-foreground/90">
            {displayValue || "null"}
          </div>
        </div>
        <CopyButton value={displayValue} />
      </div>
    </div>
  );
}

function CopyButton({ value }: { value: string }) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="xs"
      className="shrink-0"
      onClick={async () => {
        await navigator.clipboard.writeText(value);
      }}
    >
      <Copy className="size-3.5" />
      Copy
    </Button>
  );
}
