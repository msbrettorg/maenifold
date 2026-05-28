import { useMemo, useState } from "react";
import {
  FolderOpen,
  SquareArrowOutUpRight,
  SquareTerminal,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  pickWorktreePath,
  type JsonValue,
  type PipelineNode,
  type PipelineView,
} from "@/lib/pipeline";
import { type Theme } from "@/lib/theme";

import { InspectorInputs } from "./InspectorInputs";
import { InspectorLogs } from "./InspectorLogs";
import { InspectorOverview } from "./InspectorOverview";
import { InspectorOutputs } from "./InspectorOutputs";
import { InspectorRaw } from "./InspectorRaw";

type InspectorProps = {
  pipeline: PipelineView;
  node: PipelineNode | null;
  theme: Theme;
  onCancelRun: () => Promise<void>;
  isCancelling: boolean;
};

export function Inspector({
  pipeline,
  node,
  theme,
  onCancelRun,
  isCancelling,
}: InspectorProps) {
  const [tab, setTab] = useState("overview");
  const worktreePath = useMemo(
    () => pickWorktreePath(pipeline, node),
    [node, pipeline],
  );

  if (!node) {
    return (
      <div className="flex h-full items-center justify-center rounded-lg border border-dashed border-border bg-muted/30 px-6 text-center text-sm text-muted-foreground">
        Select a node to inspect its metadata, logs, and structured artifacts.
      </div>
    );
  }

  const handleOpenEditor = () => {
    if (!worktreePath) {
      return;
    }

    window.open(`vscode://file${worktreePath}`, "_blank");
  };

  const handleOpenWorktree = () => {
    if (!worktreePath) {
      return;
    }

    window.open(`file://${worktreePath}`, "_blank");
  };

  return (
    <div className="flex h-full min-h-0 flex-col rounded-xl border border-border bg-card/60">
      <Tabs
        value={tab}
        onValueChange={setTab}
        className="flex h-full min-h-0 flex-col"
      >
        <div className="border-b border-border px-4 py-3">
          <TabsList variant="line" className="w-full justify-start rounded-none p-0">
            <TabsTrigger value="overview">Overview</TabsTrigger>
            <TabsTrigger value="logs">Logs</TabsTrigger>
            <TabsTrigger value="inputs">Inputs</TabsTrigger>
            <TabsTrigger value="outputs">Outputs</TabsTrigger>
            <TabsTrigger value="raw">Raw</TabsTrigger>
          </TabsList>
        </div>

        <div className="min-h-0 flex-1 px-4 py-4">
          <TabsContent value="overview" className="h-full">
            <InspectorOverview pipeline={pipeline} node={node} />
          </TabsContent>
          <TabsContent value="logs" className="h-full">
            <InspectorLogs node={node} />
          </TabsContent>
          <TabsContent value="inputs" className="h-full">
            <InspectorInputs node={node} />
          </TabsContent>
          <TabsContent value="outputs" className="h-full">
            <InspectorOutputs node={node} />
          </TabsContent>
          <TabsContent value="raw" className="h-full">
            <InspectorRaw value={node as unknown as JsonValue} theme={theme} />
          </TabsContent>
        </div>
      </Tabs>

      <Separator />
      <div className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleOpenEditor}
            disabled={!worktreePath}
          >
            <SquareTerminal className="size-4" />
            Open in editor
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={handleOpenWorktree}
            disabled={!worktreePath}
          >
            <FolderOpen className="size-4" />
            Open worktree
          </Button>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          disabled={!pipeline.running || isCancelling}
          onClick={() => void onCancelRun()}
        >
          <SquareArrowOutUpRight className="size-4" />
          {isCancelling ? "Cancelling…" : "Cancel run"}
        </Button>
      </div>
    </div>
  );
}
