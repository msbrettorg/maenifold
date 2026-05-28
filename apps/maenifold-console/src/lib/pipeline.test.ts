import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  computeRunMetadata,
  countFailedNodes,
  deriveNodeDuration,
  getNodeTone,
  parseGatewayToolResult,
  selectPrimaryPipelineId,
  type GatewayToolResult,
  type PipelineListResponse,
  type PipelineTaskList,
  type PipelineView,
} from "./pipeline";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

function readFixture<T>(name: string): T {
  return JSON.parse(
    readFileSync(path.resolve(__dirname, "..", "__fixtures__", name), "utf8"),
  ) as T;
}

function makeGatewayToolResult(
  payload: unknown,
  overrides: Partial<GatewayToolResult> = {},
): GatewayToolResult {
  return {
    content: [{ type: "text", text: JSON.stringify(payload) }],
    ...overrides,
  };
}

describe("pipeline adapters", () => {
  const pipelineListFixture = readFixture<{ success: true; data: PipelineListResponse }>(
    "pipeline-list.fixture.json",
  );
  const pipelineViewFixture = readFixture<{ success: true; data: PipelineView }>(
    "pipeline-view.fixture.json",
  );
  const emptyViewFixture = readFixture<{ data: PipelineView }>("pipeline-view-empty.fixture.json");
  const malformedViewFixture = readFixture<{
    data: { status: number; running: string; ready: Record<string, never> };
  }>("pipeline-view-malformed.fixture.json");
  const taskListFixture = readFixture<PipelineTaskList>("task-list.fixture.json");

  it("parses the real Pipeline { sessionId, view: true } envelope", () => {
    const parsed = parseGatewayToolResult<PipelineView>(makeGatewayToolResult(pipelineViewFixture));

    expect(parsed.pipelineId).toBe("consumption-strike-synopsys-1779458959615");
    expect(parsed.status).toBe("active");
    expect(parsed.nodes.length).toBeGreaterThan(0);
    expect(parsed.edges.length).toBeGreaterThan(0);
  });

  it("parses the real Pipeline { list: true } envelope", () => {
    const parsed = parseGatewayToolResult<PipelineListResponse>(
      makeGatewayToolResult(pipelineListFixture),
    );

    expect(parsed.pipelines.length).toBeGreaterThan(0);
    expect(parsed.pipelines[0]?.pipelineId).toBe("consumption-strike-synopsys-1779458959615");
    expect(parsed.pipelines[0]?.nodes.length).toBeGreaterThan(0);
  });

  it("parses the real TaskList {} envelope", () => {
    const parsed = parseGatewayToolResult<PipelineTaskList>(makeGatewayToolResult(taskListFixture));

    expect(parsed.tasks.length).toBeGreaterThan(0);
    expect(parsed.tasks[0]).toMatchObject({
      taskId: "consumption-strike-synopsys-1779458959615",
      status: "working",
      pipelineStatus: "active",
    });
  });

  it("throws a typed Error when text content is missing", () => {
    expect(() => parseGatewayToolResult({ content: [{ type: "text" }] })).toThrowError(
      "Gateway response did not include text content",
    );
  });

  it("throws the daemon error when the tool result is flagged as an error", () => {
    expect(() =>
      parseGatewayToolResult(
        makeGatewayToolResult({ ok: false }, { isError: true, content: [{ type: "text", text: "boom" }] }),
      ),
    ).toThrowError("boom");
  });

  it("throws when the gateway response text starts with an invocation error", () => {
    expect(() =>
      parseGatewayToolResult({
        content: [{ type: "text", text: "Error invoking Pipeline" }],
      }),
    ).toThrowError("Error invoking Pipeline");
  });

  it("throws a typed Error for success:false envelopes", () => {
    expect(() =>
      parseGatewayToolResult(
        makeGatewayToolResult({ success: false, error: "gateway request failed" }),
      ),
    ).toThrowError("gateway request failed");
  });

  it("prefers the active task id over pipeline list state", () => {
    expect(selectPrimaryPipelineId(taskListFixture, pipelineListFixture.data)).toBe(
      "consumption-strike-synopsys-1779458959615",
    );
  });

  it("falls back to the active pipeline when there is no working task", () => {
    const completedTasks: PipelineTaskList = {
      tasks: taskListFixture.tasks.map((task) => ({ ...task, status: "completed" })),
      nextCursor: null,
    };

    expect(selectPrimaryPipelineId(completedTasks, pipelineListFixture.data)).toBe(
      "consumption-strike-synopsys-1779458959615",
    );
  });

  it("returns null when no active task or pipeline is available", () => {
    expect(selectPrimaryPipelineId(null, { pipelines: [] })).toBeNull();
    expect(selectPrimaryPipelineId(undefined, undefined)).toBeNull();
  });

  it.each([
    ["pending", "pending"],
    ["ready", "ready"],
    ["active", "active"],
    ["running", "active"],
    ["retrying", "active"],
    ["completed", "done"],
    ["skipped", "done"],
    ["failed", "failed"],
    ["stuck", "failed"],
    ["abandoned", "failed"],
    ["cancelled", "failed"],
  ])("maps %s to %s", (status, tone) => {
    expect(getNodeTone(status)).toBe(tone);
  });

  it("degrades unknown statuses to pending", () => {
    expect(getNodeTone("mystery-state")).toBe("pending");
  });

  it("returns safe run metadata defaults when no pipeline is present", () => {
    expect(computeRunMetadata(null)).toEqual({
      activeNodes: 0,
      completedNodes: 0,
      failedNodes: 0,
      elapsedMs: 0,
      hasActiveRun: false,
    });
  });

  it("returns safe zeroed metadata for an empty pipeline view fixture", () => {
    const metadata = computeRunMetadata(emptyViewFixture.data);

    expect(metadata.activeNodes).toBe(0);
    expect(metadata.completedNodes).toBe(0);
    expect(metadata.failedNodes).toBe(0);
    expect(metadata.hasActiveRun).toBe(false);
    expect(metadata.elapsedMs).toBeGreaterThanOrEqual(0);
  });

  it("preserves malformed inner fields so callers can degrade unknown values safely", () => {
    const parsed = parseGatewayToolResult<typeof malformedViewFixture.data>(
      makeGatewayToolResult(malformedViewFixture),
    );

    expect(parsed.status).toBe(123);
    expect(getNodeTone(String(parsed.status))).toBe("pending");
    expect(Array.isArray(parsed.ready)).toBe(false);
  });

  it("counts failed nodes and derives duration strings from the real pipeline fixture", () => {
    const failedNodes = countFailedNodes(pipelineViewFixture.data.nodes);
    const firstNode = pipelineViewFixture.data.nodes[0];

    expect(failedNodes).toBeGreaterThanOrEqual(0);
    expect(deriveNodeDuration(firstNode)).toBe("—");
  });
});
