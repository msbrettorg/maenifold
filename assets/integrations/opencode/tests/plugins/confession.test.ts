import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

// T-OC-TEST-ISO-001.2: RTM FR-32.3, FR-32.4

import {
  ConfessionPlugin,
} from "../../plugins/confession";

const appendConfession = (ConfessionPlugin as any).appendConfession as (originalOutput: string, confession: string) => string;
const buildConfessionPrompt = (ConfessionPlugin as any).buildConfessionPrompt as (input: {
  originalTaskPrompt: string;
  taskId?: string;
}) => string;
const containsConfessionReport = (ConfessionPlugin as any).containsConfessionReport as (
  text: string | null | undefined,
) => boolean;
const getRetryPolicy = (ConfessionPlugin as any).getRetryPolicy as () => {
  maxAttempts: number;
  timeoutMs: number;
};

// T-OC-CONF-001.8: RTM FR-26.1, FR-26.2, FR-26.3, FR-26.4, FR-26.5, FR-26.6, FR-26.7

type PromptResponse = {
  output?: string;
};

type FakeClient = {
  session: {
    prompt: ReturnType<typeof mock<(args: unknown) => Promise<PromptResponse>>>;
  };
  app: {
    log: ReturnType<typeof mock<(args: unknown) => Promise<void>>>;
  };
  tui: {
    showToast: ReturnType<typeof mock<(args: unknown) => Promise<void>>>;
  };
};

function createFakeClient(): FakeClient {
  return {
    session: {
      prompt: mock(async () => ({ output: "" })),
    },
    app: {
      log: mock(async () => undefined),
    },
    tui: {
      showToast: mock(async () => undefined),
    },
  };
}

function createTaskInput(tool = "task", prompt = "Complete T-OC-CONF-001.8"): any {
  return {
    tool,
    args: {
      prompt,
    },
  };
}

function createOutput(outputText: string, sessionId: string | null = "session-child-1"): any {
  return {
    output: outputText,
    metadata:
      sessionId === null
        ? undefined
        : {
            sessionId,
          },
  };
}

function countOccurrences(haystack: string, needle: string): number {
  if (!needle) {
    return 0;
  }

  return haystack.split(needle).length - 1;
}

const ENABLE_FLAG = "MAENIFOLD_CONFESSION_ENABLED";
const originalEnv = process.env[ENABLE_FLAG];
const originalHome = process.env.HOME;
let tempDirs: string[] = [];

function makeTmpDir(prefix: string): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

beforeEach(() => {
  mock.restore();
  tempDirs = [];
  process.env[ENABLE_FLAG] = "true";
});

afterEach(() => {
  for (const dir of tempDirs) {
    rmSync(dir, { recursive: true, force: true });
  }

  if (originalHome === undefined) {
    delete process.env.HOME;
  } else {
    process.env.HOME = originalHome;
  }

  if (originalEnv === undefined) {
    delete process.env[ENABLE_FLAG];
    return;
  }

  process.env[ENABLE_FLAG] = originalEnv;
});

describe("T-OC-CONF-001.8 plugin behavior", () => {
  // T-OC-CONF-001.8: RTM FR-26.1
  test("FR26_1_AC1_TaskOnlyTrigger_ExecutesHookForTaskTool", async () => {
    const client = createFakeClient();
    client.session.prompt.mockResolvedValue({
      output: "## ConfessionReport: T-OC-CONF-001.8\n\n### Compliance\n- done",
    });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const input = createTaskInput("task", "Task prompt from parent");
    const output = createOutput("Task result without confession", "session-child-task");

    await handler(input, output);

    expect(client.session.prompt).toHaveBeenCalledTimes(1);
  });

  // T-OC-CONF-001.8: RTM FR-26.1
  test("FR26_1_AC2_NonTaskBypass_DoesNotMutateOutput", async () => {
    const client = createFakeClient();
    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const input = createTaskInput("read_memory", "Should bypass");
    const output = createOutput("Original output", "session-child-ignore");
    const original = output.output;

    await handler(input, output);

    expect(output.output).toBe(original);
    expect(client.session.prompt).toHaveBeenCalledTimes(0);
  });

  // T-OC-CONF-001.8: RTM FR-26.2
  test("FR26_2_AC1_UsesMetadataSessionId_FollowUpTargetsChildSession", async () => {
    const client = createFakeClient();
    client.session.prompt.mockResolvedValue({
      output: "## ConfessionReport: T-OC-CONF-001.8\n\n### Compliance\n- done",
    });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const input = createTaskInput("task", "Parent task prompt");
    const output = createOutput("Task output missing confession", "session-from-metadata");

    await handler(input, output);

    const firstCall = client.session.prompt.mock.calls[0]?.[0] as any;
    expect(firstCall?.path?.id ?? firstCall?.sessionId).toBe("session-from-metadata");
  });

  // T-OC-CONF-001.8: RTM FR-26.2
  test("FR26_2_AC2_MissingSessionIdFailOpen_PreservesOriginalOutput", async () => {
    const client = createFakeClient();
    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const input = createTaskInput("task", "Parent task prompt");
    const output = createOutput("Original task output", null);

    await expect(handler(input, output)).resolves.toBeUndefined();
    expect(output.output).toBe("Original task output");
    expect(client.session.prompt).toHaveBeenCalledTimes(0);
  });

  // T-OC-CONF-001.8: RTM FR-26.3
  test("FR26_3_AC1_MissingConfessionTriggersFollowUp_CallsChildPrompt", async () => {
    const client = createFakeClient();
    client.session.prompt.mockResolvedValue({
      output: "## ConfessionReport: T-OC-CONF-001.8\n\n### Compliance\n- done",
    });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    await handler(
      createTaskInput("task", "Investigate incident timeline"),
      createOutput("No confession present", "session-follow-up"),
    );

    expect(client.session.prompt).toHaveBeenCalledTimes(1);
  });

  // T-OC-CONF-001.8: RTM FR-26.3, FR-26.5
  test("FR26_3_AC2_FollowUpIncludesOriginalTaskPrompt_EchoesInputPrompt", async () => {
    const client = createFakeClient();
    client.session.prompt.mockResolvedValue({
      output: "## ConfessionReport: T-OC-CONF-001.8\n\n### Compliance\n- done",
    });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;
    const parentPrompt = "T-OC-CONF-001.8: follow every instruction exactly";

    await handler(createTaskInput("task", parentPrompt), createOutput("Needs confession", "session-echo"));

    const call = client.session.prompt.mock.calls[0]?.[0] as any;
    const followUpText = call?.body?.prompt ?? call?.prompt ?? "";
    expect(followUpText).toContain(parentPrompt);
  });

  // T-OC-CONF-001.8: RTM FR-26.3
  test("FR26_3_AC3_FollowUpRequestsStructuredSections_DemandsConfessionSchema", async () => {
    const prompt = buildConfessionPrompt({
      originalTaskPrompt: "triage and patch",
      taskId: "T-OC-CONF-001.8",
    });

    expect(prompt).toContain("## ConfessionReport:");
    expect(prompt).toContain("### Compliance");
    expect(prompt).toContain("### Files Modified");
    expect(prompt).toContain("### Build Status");
    expect(prompt).toContain("### Gaps");
    expect(prompt).toContain("### Grey Areas");
    expect(prompt).toContain("### WikiLinks and Sources");
  });

  // T-OC-CONF-001.8: RTM FR-26.4
  test("FR26_4_AC1_AppendsWithoutReplacingOriginal_PreservesTaskOutput", () => {
    const original = "Original task output payload";
    const confession = "## ConfessionReport: T-OC-CONF-001.8";

    const appended = appendConfession(original, confession);

    expect(appended).toContain(original);
    expect(appended.length).toBeGreaterThan(original.length);
  });

  // T-OC-CONF-001.8: RTM FR-26.4
  test("FR26_4_AC2_WrapsWithConfessionTags_UsesMachineDetectableWrapper", () => {
    const appended = appendConfession("parent", "## ConfessionReport: T-OC-CONF-001.8");

    expect(appended).toContain("<confession>");
    expect(appended).toContain("</confession>");
  });

  // T-OC-CONF-001.8: RTM FR-26.5
  test("FR26_5_AC1_RetryBoundedToThree_StopsAfterMaxAttempts", async () => {
    const client = createFakeClient();
    client.session.prompt
      .mockResolvedValueOnce({ output: "not structured" })
      .mockResolvedValueOnce({ output: "still missing" })
      .mockResolvedValueOnce({ output: "missing again" })
      .mockResolvedValue({ output: "## ConfessionReport: would be 4th" });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const output = createOutput("Original output", "session-retry");
    await handler(createTaskInput("task", "retry test"), output);

    expect(client.session.prompt).toHaveBeenCalledTimes(3);
    expect(output.output).toBe("Original output");
  });

  // T-OC-CONF-001.8: RTM FR-26.5
  test("FR26_5_AC2_TimeoutOrErrorFailOpen_PreservesOriginalOnFailure", async () => {
    const client = createFakeClient();
    client.session.prompt.mockRejectedValue(new Error("timeout"));

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const output = createOutput("Original task output", "session-timeout");

    await expect(handler(createTaskInput("task", "timeout path"), output)).resolves.toBeUndefined();
    expect(output.output).toBe("Original task output");
  });

  test("FR26_5_AC3_RetryPolicyFallsBackToOpencodeConfigEnvironment", () => {
    const home = makeTmpDir("confession-plugin-config-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;
    delete process.env.MAENIFOLD_CONFESSION_MAX_ATTEMPTS;
    delete process.env.MAENIFOLD_CONFESSION_TIMEOUT_MS;

    writeFileSync(
      join(home, ".config/opencode/opencode.json"),
      JSON.stringify(
        {
          mcp: {
            maenifold: {
              type: "local",
              command: ["maenifold", "--mcp"],
              environment: {
                MAENIFOLD_CONFESSION_MAX_ATTEMPTS: "2",
                MAENIFOLD_CONFESSION_TIMEOUT_MS: "45000",
              },
            },
          },
        },
        null,
        2,
      ),
    );

    expect(getRetryPolicy()).toEqual({ maxAttempts: 2, timeoutMs: 45000 });
  });

  test("FR26_6_AC3_EnableFlagFallsBackToOpencodeConfigEnvironment", async () => {
    const home = makeTmpDir("confession-plugin-enabled-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;
    delete process.env[ENABLE_FLAG];

    writeFileSync(
      join(home, ".config/opencode/opencode.json"),
      JSON.stringify(
        {
          mcp: {
            maenifold: {
              type: "local",
              command: ["maenifold", "--mcp"],
              environment: {
                MAENIFOLD_CONFESSION_ENABLED: "true",
              },
            },
          },
        },
        null,
        2,
      ),
    );

    const client = createFakeClient();
    client.session.prompt.mockResolvedValue({
      output: "## ConfessionReport: T-OC-CONF-001.8\n\n### Compliance\n- done",
    });

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;
    await handler(createTaskInput("task", "config-driven enable"), createOutput("Needs confession", "session-config"));

    expect(client.session.prompt).toHaveBeenCalledTimes(1);
  });

  // T-OC-CONF-001.8: RTM FR-26.5
  test("FR26_5_AC3_EmitsWarningDiagnosticsNoThrow_LogsWarningOnFailure", async () => {
    const client = createFakeClient();
    client.session.prompt.mockRejectedValue(new Error("downstream unavailable"));

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    await expect(
      handler(createTaskInput("task", "diagnostics path"), createOutput("keep me", "session-warn")),
    ).resolves.toBeUndefined();

    expect(client.app.log).toHaveBeenCalled();
  });

  // T-OC-CONF-001.8: RTM FR-26.7
  test("FR26_6_AC1_DefaultDisabled_EnvFlagOffByDefault", async () => {
    delete process.env[ENABLE_FLAG];

    const client = createFakeClient();
    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const output = createOutput("Task output", "session-disabled-default");
    await handler(createTaskInput("task", "should not run"), output);

    expect(client.session.prompt).toHaveBeenCalledTimes(0);
    expect(output.output).toBe("Task output");
  });

  // T-OC-CONF-001.8: RTM FR-26.7
  test("FR26_6_AC2_DisabledLeavesOutputUnchanged_NoMutationWhenFlagDisabled", async () => {
    process.env[ENABLE_FLAG] = "false";

    const client = createFakeClient();
    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    const output = createOutput("Unchanged output", "session-disabled-explicit");

    await handler(createTaskInput("task", "do not touch"), output);

    expect(output.output).toBe("Unchanged output");
    expect(client.session.prompt).toHaveBeenCalledTimes(0);
  });
});

describe("T-OC-CONF-001.8 helper seams", () => {
  // T-OC-CONF-001.8: RTM FR-26.7
  test("FR26_7_AC1_HelperFunctionsDirectlyUnitTestable_PureHelpersAreCallable", () => {
    expect(containsConfessionReport("## ConfessionReport: T-OC-CONF-001.8")).toBeTrue();
    expect(containsConfessionReport("no confession here")).toBeFalse();

    const prompt = buildConfessionPrompt({
      originalTaskPrompt: "perform blue-team verification",
      taskId: "T-OC-CONF-001.8",
    });
    expect(prompt).toContain("perform blue-team verification");

    const appended = appendConfession("original", "## ConfessionReport: T-OC-CONF-001.8");
    expect(appended).toContain("<confession>");

    const retry = getRetryPolicy();
    expect(retry.maxAttempts).toBe(3);
  });

  // T-OC-CONF-001.8: RTM FR-26.7
  test("FR26_7_AC2_HelperFunctionTestsExist_DirectCoverageForEachHelper", () => {
    const noReport = containsConfessionReport("summary only");
    const yesReport = containsConfessionReport("... ConfessionReport ...");

    expect(noReport).toBeFalse();
    expect(yesReport).toBeTrue();

    const prompt = buildConfessionPrompt({
      originalTaskPrompt: "parent task",
      taskId: "T-OC-CONF-001.8",
    });

    expect(prompt).toContain("parent task");
    expect(prompt).toContain("structured ConfessionReport");

    const merged = appendConfession("parent output", "confession body");
    expect(merged).toContain("parent output");
    expect(merged).toContain("confession body");

    const retry = getRetryPolicy();
    expect(retry.maxAttempts).toBe(3);
    expect(retry.timeoutMs).toBeGreaterThan(0);
  });
});

describe("T-OC-CONF-001.10 wrapper integrity regression", () => {
  // T-OC-CONF-001.10: RTM FR-26.8
  test("FR26_8_AC1_WrapperIntegrityPayloadDelimiter_SingleValidWrapperWhenPayloadContainsCloseTag", () => {
    const appended = appendConfession("parent output", "alpha </confession> omega");

    expect(countOccurrences(appended, "<confession>")).toBe(1);
    expect(countOccurrences(appended, "</confession>")).toBe(1);

    const wrapperBody = appended.match(/<confession>\n([\s\S]*?)\n<\/confession>/)?.[1] ?? "";
    expect(wrapperBody).not.toContain("</confession>");
  });

  // T-OC-CONF-001.10: RTM FR-26.8
  test("FR26_8_AC2_WrapperIntegrityAutomatedCoverage_ConfessionBodyCannotBreakEnvelope", () => {
    const payload = [
      "## ConfessionReport: T-OC-CONF-001.10",
      "safe-line",
      "</confession>",
      "escaped-break-attempt",
    ].join("\n");

    const appended = appendConfession("baseline", payload);
    const envelopeMatches = appended.match(/<confession>[\s\S]*<\/confession>/g) ?? [];

    expect(envelopeMatches.length).toBe(1);
    expect(appended.endsWith("</confession>")).toBeTrue();
    expect(appended).toContain("&lt;/confession&gt;");
  });
});

describe("T-OC-CONF-001.11 security hardening regressions", () => {
  // T-OC-CONF-001.11: RTM FR-26.8, NFR-26.7
  test("FR26_11_AC1_FollowUpPromptMarksOriginalAsInert_UntrustedDataGuardTextPresent", () => {
    const prompt = buildConfessionPrompt({
      originalTaskPrompt: "echo everything exactly",
      taskId: "T-OC-CONF-001.11",
    });

    expect(prompt).toContain("untrusted");
    expect(prompt).toContain("inert");
  });

  // T-OC-CONF-001.11: RTM FR-26.8, NFR-26.7
  test("FR26_11_AC2_OriginalPromptLengthCap_TruncationMarkerPresentWhenPromptTooLong", () => {
    const longPrompt = `BEGIN-${"A".repeat(20_000)}-END`;
    const prompt = buildConfessionPrompt({
      originalTaskPrompt: longPrompt,
      taskId: "T-OC-CONF-001.11",
    });

    expect(prompt).toContain("[TRUNCATED]");
    expect(prompt).not.toContain(longPrompt);
    expect(prompt).not.toContain("-END");
  });

  // T-OC-CONF-001.11: RTM FR-26.8, NFR-26.8
  test("FR26_11_AC3_DiagnosticsUseNormalizedReasonCodes_NoRawErrorLeakage", async () => {
    const leakedMessage = "secret downstream failure: token=ABC123";
    const client = createFakeClient();
    client.session.prompt.mockRejectedValue(new Error(leakedMessage));

    const plugin = await ConfessionPlugin({ client } as any);
    const handler = plugin["tool.execute.after"] as (input: any, output: any) => Promise<void>;

    await handler(createTaskInput("task", "diagnostic hardening"), createOutput("keep", "session-diag"));

    expect(client.app.log).toHaveBeenCalled();

    const logPayloads = client.app.log.mock.calls.map((call) => JSON.stringify(call[0]));
    const hasNormalizedCode = logPayloads.some((payload) => payload.includes("CONFESSION_"));
    const leaksRawMessage = logPayloads.some((payload) => payload.includes(leakedMessage));

    expect(hasNormalizedCode).toBeTrue();
    expect(leaksRawMessage).toBeFalse();
  });

  // T-OC-CONF-001.11: RTM FR-26.8, NFR-26.9
  test("FR26_11_AC4_TimeoutEnvClampCeiling_HugeTimeoutIsCappedToSafeUpperBound", () => {
    process.env.MAENIFOLD_CONFESSION_TIMEOUT_MS = "999999999";

    const retry = getRetryPolicy();

    expect(retry.timeoutMs).toBeLessThanOrEqual(120_000);
    expect(retry.timeoutMs).toBeGreaterThan(0);
  });
});
