import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

import { CompactionPlugin } from "../../plugins/compaction";

// T-OC-COMP-001.1: RTM FR-12.3, NFR-12.3.1, FR-32.1, FR-32.2
// T-OC-COMP-001.2: RTM FR-32.2, FR-32.3

type EnvSnapshot = {
  HOME?: string;
  MAENIFOLD_ROOT?: string;
};

function snapshotEnv(): EnvSnapshot {
  return {
    HOME: process.env.HOME,
    MAENIFOLD_ROOT: process.env.MAENIFOLD_ROOT,
  };
}

function restoreEnv(snapshot: EnvSnapshot): void {
  if (snapshot.HOME === undefined) {
    delete process.env.HOME;
  } else {
    process.env.HOME = snapshot.HOME;
  }

  if (snapshot.MAENIFOLD_ROOT === undefined) {
    delete process.env.MAENIFOLD_ROOT;
  } else {
    process.env.MAENIFOLD_ROOT = snapshot.MAENIFOLD_ROOT;
  }
}

function makeTmpDir(prefix: string): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

const TEST_DIR = dirname(fileURLToPath(import.meta.url));

function configureIsolatedMaenifoldRoot(tempRoot: string): string {
  const maenifoldRoot = join(tempRoot, "maenifold-root");
  mkdirSync(join(maenifoldRoot, "memory"), { recursive: true });
  process.env.HOME = tempRoot;
  process.env.MAENIFOLD_ROOT = maenifoldRoot;
  return maenifoldRoot;
}

async function runCompactionHook(initialContext: string[] = []): Promise<string[]> {
  const plugin = await CompactionPlugin({} as any);
  const handler = plugin["experimental.session.compacting"] as
    | ((input: unknown, output: { context: string[] }) => Promise<void>)
    | undefined;

  if (!handler) {
    throw new Error("Compaction plugin did not register experimental.session.compacting");
  }

  const output = { context: [...initialContext] as string[] };
  await handler({}, output);

  return output.context;
}

describe("T-OC-COMP-001 compaction plugin coverage", () => {
  const originalEnv = snapshotEnv();
  let tempDirs: string[] = [];

  beforeEach(() => {
    tempDirs = [];
    restoreEnv(originalEnv);
  });

  afterEach(() => {
    for (const dir of tempDirs) {
      rmSync(dir, { recursive: true, force: true });
    }

    restoreEnv(originalEnv);
  });

  // T-OC-COMP-001.1: RTM FR-12.3, FR-32.1
  test("FR12_3_AC1_CompactionPluginImportResolvesToCallablePluginFactory", async () => {
    expect(CompactionPlugin).toBeFunction();

    const plugin = await CompactionPlugin({} as any);

    expect(plugin).toBeObject();
  });

  // T-OC-COMP-001.1: RTM FR-12.3, FR-32.1
  test("FR12_3_AC2_RegistersExperimentalSessionCompactingHook", async () => {
    const plugin = await CompactionPlugin({} as any);

    expect(plugin["experimental.session.compacting"]).toBeFunction();
  });

  // T-OC-COMP-001.1: RTM FR-12.3
  test("FR12_3_AC3_ReadableFilterRendersBannedWikiLinksIntoInjectedContext", async () => {
    const tempRoot = makeTmpDir("compaction-plugin-readable-filter-");
    tempDirs.push(tempRoot);

    const maenifoldRoot = configureIsolatedMaenifoldRoot(tempRoot);
    writeFileSync(
      join(maenifoldRoot, "memory", ".wikilink-filter.json"),
      JSON.stringify({
        agent: "too generic",
        memory: "too broad",
        tool: "meta-term",
      }),
      "utf8",
    );

    const injectedContext = (await runCompactionHook()).join("\n");

    expect(injectedContext).toContain("BANNED: [[agent]], [[memory]], [[tool]]");
    expect(injectedContext).toContain("USE INSTEAD: Specific terms like [[authentication]], [[JWT]], [[vector-embeddings]], [[maenifold-internals]]");
  });

  // T-OC-COMP-001.1: RTM FR-12.3
  test("FR12_3_AC4_MissingFilterFallsBackFailOpenAndStillInjectsGuidance", async () => {
    const tempRoot = makeTmpDir("compaction-plugin-missing-filter-");
    tempDirs.push(tempRoot);

    configureIsolatedMaenifoldRoot(tempRoot);

    const injectedContext = (await runCompactionHook()).join("\n");

    expect(injectedContext).toContain("BANNED: (could not load filter list)");
    expect(injectedContext).toContain("please tag important concepts, entities and terms as [[wikilinks]]");
    expect(injectedContext).toContain("WikiLinks become graph nodes. Bad tagging = graph corruption = broken context recovery.");
  });

  test("FR12_3_AC4b_MaenifoldRootFallsBackToOpencodeConfigEnvironment", async () => {
    const tempRoot = makeTmpDir("compaction-plugin-config-root-");
    tempDirs.push(tempRoot);

    const configuredRoot = join(tempRoot, "configured-root");
    mkdirSync(join(configuredRoot, "memory"), { recursive: true });
    writeFileSync(
      join(configuredRoot, "memory", ".wikilink-filter.json"),
      JSON.stringify({ agent: "too generic" }),
      "utf8",
    );

    mkdirSync(join(tempRoot, ".config/opencode"), { recursive: true });
    process.env.HOME = tempRoot;
    delete process.env.MAENIFOLD_ROOT;
    writeFileSync(
      join(tempRoot, ".config/opencode/opencode.json"),
      JSON.stringify(
        {
          mcp: {
            maenifold: {
              type: "local",
              command: ["maenifold", "--mcp"],
              environment: {
                MAENIFOLD_ROOT: configuredRoot,
              },
            },
          },
        },
        null,
        2,
      ),
      "utf8",
    );

    const injectedContext = (await runCompactionHook()).join("\n");
    expect(injectedContext).toContain("BANNED: [[agent]]");
  });

  // T-OC-COMP-001.1: RTM FR-12.3, NFR-12.3.1
  test("FR12_3_AC5_InjectedContextIncludesNormalizationRulesAndAntiPatternGuidance", async () => {
    const tempRoot = makeTmpDir("compaction-plugin-guidance-");
    tempDirs.push(tempRoot);

    configureIsolatedMaenifoldRoot(tempRoot);

    const injectedContext = (await runCompactionHook()).join("\n");

    expect(injectedContext).toContain("Double brackets: [[WikiLink]] never [WikiLink]");
    expect(injectedContext).toContain("Normalized to lowercase-with-hyphens internally");
    expect(injectedContext).toContain("HYPHENATE multiword: [[null-reference-exception]] not [[Null Reference Exception]]");
    expect(injectedContext).toContain("Anti-patterns (silently normalized but avoid):");
    expect(injectedContext).toContain("Underscores: [[my_concept]]");
    expect(injectedContext).toContain("Slashes: [[foo/bar]]");
    expect(injectedContext).toContain("Double hyphens: [[foo--bar]]");
    expect(injectedContext).toContain("Leading/trailing hyphens: [[-concept-]]");
  });

  // T-OC-COMP-001.1: RTM FR-12.3, NFR-12.3.1, FR-32.1, FR-32.2
  test("FR12_3_AC6_PreservesExistingOutputContextAndAppendsExactlyOneGuidanceBlock", async () => {
    const tempRoot = makeTmpDir("compaction-plugin-append-semantics-");
    tempDirs.push(tempRoot);

    configureIsolatedMaenifoldRoot(tempRoot);

    const originalContextEntry = "existing context entry";
    const outputContext = await runCompactionHook([originalContextEntry]);
    const injectedGuidance = outputContext[1];

    expect(outputContext[0]).toBe(originalContextEntry);
    expect(outputContext).toHaveLength(2);
    expect(injectedGuidance).toContain(
      "When writing your summaries for compaction/continuation",
    );
    expect(
      outputContext.filter((entry) => entry === injectedGuidance),
    ).toHaveLength(1);
  });

  // T-OC-COMP-001.2: RTM FR-32.2
  test("FR32_2_AC1_CompactionSuiteFileExists", () => {
    expect(existsSync(join(TEST_DIR, "compaction.test.ts"))).toBe(true);
  });

  // T-OC-COMP-001.2: RTM FR-32.2, FR-32.3
  test("FR32_2_AC2_ConfessionSuiteFileExists", () => {
    expect(existsSync(join(TEST_DIR, "confession.test.ts"))).toBe(true);
  });

  // T-OC-COMP-001.2: RTM FR-32.2, FR-32.3
  test("FR32_2_AC3_PersistenceSuiteFileExists", () => {
    expect(existsSync(join(TEST_DIR, "persistence.test.ts"))).toBe(true);
  });

  // T-OC-COMP-001.2: RTM FR-32.2, FR-32.3
  test("FR32_3_AC4_RuntimePluginSetMatchesDedicatedSuiteSet", () => {
    const pluginDir = join(TEST_DIR, "../../plugins");
    const testDir = TEST_DIR;

    const runtimePlugins = readdirSync(pluginDir)
      .filter((file) => file.endsWith(".ts"))
      .map((file) => basename(file, ".ts"))
      .sort();

    const dedicatedPluginSuites = readdirSync(testDir)
      .filter((file) => file.endsWith(".test.ts"))
      .map((file) => basename(file, ".test.ts"))
      .sort();

    expect(runtimePlugins).toEqual(["compaction", "confession", "persistence"]);
    expect(dedicatedPluginSuites).toEqual(runtimePlugins);
  });
});
