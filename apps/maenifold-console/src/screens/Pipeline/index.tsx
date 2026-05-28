import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, LoaderCircle } from "lucide-react";

import { Alert, AlertAction, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { Skeleton } from "@/components/ui/skeleton";
import { usePolledQuery } from "@/hooks/usePolledQuery";
import { useTheme } from "@/hooks/useTheme";
import { useBridge } from "@/lib/bridge/useBridge";
import {
  computeRunMetadata,
  normalizePipelineList,
  normalizePipelineView,
  normalizeTaskList,
  selectPrimaryPipelineId,
} from "@/lib/pipeline";

import { Dag } from "./Dag";
import { Inspector } from "./Inspector";
import { StatStrip } from "./StatStrip";
import { StatusBar } from "./StatusBar";
import { TopBar } from "./TopBar";

export default function PipelineScreen() {
  const bridge = useBridge();
  const { theme, toggleTheme } = useTheme();
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [isCancelling, setIsCancelling] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const [mountedAt] = useState(() => Date.now());

  useEffect(() => {
    const interval = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, []);

  const invokeGateway = useCallback(
    (domain: string, command: string, parameters: Record<string, unknown>) =>
      bridge.invoke("GatewayExecute", {
        domain,
        command,
        parameters,
      }),
    [bridge],
  );

  const tasksQuery = usePolledQuery(
    "tasks",
    async () =>
      normalizeTaskList(
        await invokeGateway("maenifold.tasks", "TaskList", {}),
      ),
    { intervalMs: 2_000 },
  );
  const pipelineListQuery = usePolledQuery(
    "pipeline-list",
    async () =>
      normalizePipelineList(
        await invokeGateway("maenifold.workflow", "Pipeline", { list: true }),
      ),
    { intervalMs: 2_000 },
  );

  const activePipelineId = useMemo(
    () => selectPrimaryPipelineId(tasksQuery.data, pipelineListQuery.data),
    [pipelineListQuery.data, tasksQuery.data],
  );

  const pipelineViewQuery = usePolledQuery(
    activePipelineId ?? "pipeline-view-idle",
    () =>
      activePipelineId
        ? invokeGateway("maenifold.workflow", "Pipeline", {
            sessionId: activePipelineId,
            view: true,
          }).then((result) => normalizePipelineView(result))
        : Promise.resolve(null),
    {
      enabled: Boolean(activePipelineId),
      intervalMs: 2_000,
      initialData: null,
    },
  );

  const pipeline = pipelineViewQuery.data;
  const selectedNode = useMemo(
    () =>
      pipeline?.nodes.find((node) => node.nodeId === selectedNodeId) ?? null,
    [pipeline, selectedNodeId],
  );

  useEffect(() => {
    if (!pipeline) {
      setSelectedNodeId(null);
      return;
    }

    const stillExists = pipeline.nodes.some((node) => node.nodeId === selectedNodeId);
    if (stillExists) {
      return;
    }

    const nextNode =
      pipeline.nodes.find((node) => pipeline.active.includes(node.nodeId)) ??
      pipeline.nodes.find((node) => pipeline.ready.includes(node.nodeId)) ??
      pipeline.nodes[0] ??
      null;

    setSelectedNodeId(nextNode?.nodeId ?? null);
  }, [pipeline, selectedNodeId]);

  const metadata = useMemo(() => computeRunMetadata(pipeline, now), [now, pipeline]);
  const [activeHistory, completedHistory, failedHistory, elapsedHistory] = useMetricHistories(metadata);

  const error =
    tasksQuery.error ??
    pipelineListQuery.error ??
    pipelineViewQuery.error;

  const isInitialLoading =
    tasksQuery.isLoading ||
    pipelineListQuery.isLoading ||
    (Boolean(activePipelineId) && pipelineViewQuery.isLoading && !pipeline);

  const isHealthy = Boolean(
    !error &&
      (pipelineViewQuery.lastUpdatedAt ??
        pipelineListQuery.lastUpdatedAt ??
        tasksQuery.lastUpdatedAt),
  );

  const lastHeartbeatAt =
    pipelineViewQuery.lastUpdatedAt ??
    pipelineListQuery.lastUpdatedAt ??
    tasksQuery.lastUpdatedAt;

  const handleRefresh = useCallback(async () => {
    await Promise.all([
      tasksQuery.refetch(),
      pipelineListQuery.refetch(),
      pipelineViewQuery.refetch(),
    ]);
  }, [pipelineListQuery, pipelineViewQuery, tasksQuery]);

  const handleCancelRun = useCallback(async () => {
    if (!pipeline) {
      return;
    }

    setIsCancelling(true);
    try {
      await invokeGateway("maenifold.workflow", "Pipeline", {
        sessionId: pipeline.pipelineId,
        cancel: true,
      });
      await handleRefresh();
    } finally {
      setIsCancelling(false);
    }
  }, [handleRefresh, invokeGateway, pipeline]);

  return (
    <div className="flex min-h-screen flex-col bg-background text-foreground">
      <TopBar
        pipelineName={pipeline?.name ?? activePipelineId}
        pipelineId={activePipelineId}
        theme={theme}
        toggleTheme={toggleTheme}
        onRefresh={handleRefresh}
      />

      <main className="flex min-h-0 flex-1 flex-col gap-4 p-4">
        {isInitialLoading ? (
          <>
            <LoadingStatStrip />
            <LoadingPanels />
          </>
        ) : error ? (
          <>
            <StatStrip
              metadata={metadata}
              activeHistory={activeHistory}
              completedHistory={completedHistory}
              failedHistory={failedHistory}
              elapsedHistory={elapsedHistory}
            />
            <ErrorState error={error} onRetry={handleRefresh} />
          </>
        ) : !pipeline ? (
          <>
            <StatStrip
              metadata={metadata}
              activeHistory={activeHistory}
              completedHistory={completedHistory}
              failedHistory={failedHistory}
              elapsedHistory={elapsedHistory}
            />
            <EmptyState />
          </>
        ) : (
          <>
            <StatStrip
              metadata={metadata}
              activeHistory={activeHistory}
              completedHistory={completedHistory}
              failedHistory={failedHistory}
              elapsedHistory={elapsedHistory}
            />
            <ResizablePanelGroup direction="horizontal" className="min-h-0 flex-1 gap-3">
              <ResizablePanel defaultSize={62} minSize={42}>
                <Dag
                  pipeline={pipeline}
                  selectedNodeId={selectedNodeId}
                  onSelectNode={setSelectedNodeId}
                />
              </ResizablePanel>
              <ResizableHandle withHandle className="rounded-full bg-border/80" />
              <ResizablePanel defaultSize={38} minSize={28}>
                <Inspector
                  pipeline={pipeline}
                  node={selectedNode}
                  theme={theme}
                  onCancelRun={handleCancelRun}
                  isCancelling={isCancelling}
                />
              </ResizablePanel>
            </ResizablePanelGroup>
          </>
        )}
      </main>

      <StatusBar
        metadata={metadata}
        transport={bridge.transport}
        isHealthy={isHealthy}
        uptimeMs={now - mountedAt}
        lastHeartbeatAt={lastHeartbeatAt}
      />
    </div>
  );
}

function useMetricHistories(metadata: ReturnType<typeof computeRunMetadata>) {
  const [activeHistory, setActiveHistory] = useState<number[]>([]);
  const [completedHistory, setCompletedHistory] = useState<number[]>([]);
  const [failedHistory, setFailedHistory] = useState<number[]>([]);
  const [elapsedHistory, setElapsedHistory] = useState<number[]>([]);

  useEffect(() => {
    const append = (history: number[], value: number) => [...history.slice(-19), value];

    setActiveHistory((history) => append(history, metadata.activeNodes));
    setCompletedHistory((history) => append(history, metadata.completedNodes));
    setFailedHistory((history) => append(history, metadata.failedNodes));
    setElapsedHistory((history) => append(history, metadata.elapsedMs / 1000));
  }, [metadata.activeNodes, metadata.completedNodes, metadata.elapsedMs, metadata.failedNodes]);

  return [activeHistory, completedHistory, failedHistory, elapsedHistory] as const;
}

function LoadingStatStrip() {
  return (
    <div className="grid gap-3 md:grid-cols-4">
      {Array.from({ length: 4 }).map((_, index) => (
        <Skeleton key={index} className="h-14 rounded-xl" />
      ))}
    </div>
  );
}

function LoadingPanels() {
  return (
    <div className="grid min-h-0 flex-1 gap-3 md:grid-cols-[1.63fr_1fr]">
      <Skeleton className="h-full min-h-[24rem] rounded-xl" />
      <Skeleton className="h-full min-h-[24rem] rounded-xl" />
    </div>
  );
}

function EmptyState() {
  return (
    <Card className="flex min-h-[24rem] flex-1 items-center justify-center border-dashed">
      <CardContent className="space-y-3 text-center">
        <div className="font-mono text-[11px] uppercase tracking-[0.18em] text-muted-foreground">
          No active pipeline
        </div>
        <div className="text-lg font-semibold tracking-tight">
          The daemon is reachable, but no active pipeline is currently running.
        </div>
        <div className="text-sm text-muted-foreground">
          When the next run starts, the DAG and inspector will populate automatically.
        </div>
      </CardContent>
    </Card>
  );
}

function ErrorState({
  error,
  onRetry,
}: {
  error: Error;
  onRetry: () => Promise<void>;
}) {
  return (
    <Alert variant="destructive" className="min-h-[12rem] items-start">
      <AlertCircle className="mt-0.5 size-4" />
      <AlertTitle>Unable to load pipeline data</AlertTitle>
      <AlertDescription>{error.message}</AlertDescription>
      <AlertAction>
        <Button
          type="button"
          size="sm"
          variant="ghost"
          className="text-destructive hover:bg-destructive/10 hover:text-destructive"
          onClick={() => void onRetry()}
        >
          <LoaderCircle className="size-4" />
          Retry
        </Button>
      </AlertAction>
    </Alert>
  );
}
