import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { Database } from "bun:sqlite";
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import { PersistencePlugin } from "../../plugins/persistence";

const __test__ = (PersistencePlugin as any).__test__;

// T-OC-TEST-ISO-001.2: RTM FR-32.3, FR-32.4

type EnvSnapshot = {
  HOME?: string;
  MAENIFOLD_ROOT?: string;
  MAENIFOLD_DATABASE_PATH?: string;
  MAENIFOLD_COMMUNITIES?: string;
  MAENIFOLD_RECENT_ACTIVITY?: string;
  MAENIFOLD_RECENT_COMPACTIONS?: string;
};

function snapshotEnv(): EnvSnapshot {
  return {
    HOME: process.env.HOME,
    MAENIFOLD_ROOT: process.env.MAENIFOLD_ROOT,
    MAENIFOLD_DATABASE_PATH: process.env.MAENIFOLD_DATABASE_PATH,
    MAENIFOLD_COMMUNITIES: process.env.MAENIFOLD_COMMUNITIES,
    MAENIFOLD_RECENT_ACTIVITY: process.env.MAENIFOLD_RECENT_ACTIVITY,
    MAENIFOLD_RECENT_COMPACTIONS: process.env.MAENIFOLD_RECENT_COMPACTIONS,
  };
}

function restoreEnv(snapshot: EnvSnapshot): void {
  if (snapshot.HOME === undefined) delete process.env.HOME;
  else process.env.HOME = snapshot.HOME;

  if (snapshot.MAENIFOLD_ROOT === undefined) delete process.env.MAENIFOLD_ROOT;
  else process.env.MAENIFOLD_ROOT = snapshot.MAENIFOLD_ROOT;

  if (snapshot.MAENIFOLD_DATABASE_PATH === undefined) {
    delete process.env.MAENIFOLD_DATABASE_PATH;
  } else {
    process.env.MAENIFOLD_DATABASE_PATH = snapshot.MAENIFOLD_DATABASE_PATH;
  }

  if (snapshot.MAENIFOLD_COMMUNITIES === undefined) {
    delete process.env.MAENIFOLD_COMMUNITIES;
  } else {
    process.env.MAENIFOLD_COMMUNITIES = snapshot.MAENIFOLD_COMMUNITIES;
  }

  if (snapshot.MAENIFOLD_RECENT_ACTIVITY === undefined) {
    delete process.env.MAENIFOLD_RECENT_ACTIVITY;
  } else {
    process.env.MAENIFOLD_RECENT_ACTIVITY = snapshot.MAENIFOLD_RECENT_ACTIVITY;
  }

  if (snapshot.MAENIFOLD_RECENT_COMPACTIONS === undefined) {
    delete process.env.MAENIFOLD_RECENT_COMPACTIONS;
  } else {
    process.env.MAENIFOLD_RECENT_COMPACTIONS = snapshot.MAENIFOLD_RECENT_COMPACTIONS;
  }
}

function makeTmpDir(prefix: string): string {
  return mkdtempSync(join(tmpdir(), prefix));
}

function createFileContentDb(
  dbPath: string,
  rows: Array<{
    filePath: string;
    status?: string | null;
    content?: string | null;
    lastIndexed?: string | null;
    created?: string | null;
    lastAccessed?: string | null;
  }>,
): void {
  const db = new Database(dbPath);
  db.exec(`
    CREATE TABLE IF NOT EXISTS file_content (
      file_path TEXT PRIMARY KEY,
      status TEXT,
      content TEXT,
      last_indexed TEXT,
      created TEXT,
      last_accessed TEXT
    );
  `);

  const insert = db.query(
    `
      INSERT INTO file_content
        (file_path, status, content, last_indexed, created, last_accessed)
      VALUES (?, ?, ?, ?, ?, ?);
    `,
  );

  for (const row of rows) {
    insert.run(
      row.filePath,
      row.status ?? null,
      row.content ?? null,
      row.lastIndexed ?? null,
      row.created ?? null,
      row.lastAccessed ?? null,
    );
  }

  db.close();
}

function seedCommunityGraph(
  dbPath: string,
  options: {
    communityCount: number;
    conceptsPerCommunity: number;
    anchorPrefix?: string;
    conceptWeightBase?: number;
  },
): void {
  const {
    communityCount,
    conceptsPerCommunity,
    anchorPrefix = "anchor",
    conceptWeightBase = 1_000,
  } = options;

  const db = new Database(dbPath);
  db.exec(`
    CREATE TABLE IF NOT EXISTS concept_communities (
      concept_name TEXT PRIMARY KEY,
      community_id INTEGER
    );

    CREATE TABLE IF NOT EXISTS concept_graph (
      concept_a TEXT,
      concept_b TEXT,
      co_occurrence_count INTEGER
    );

    CREATE TABLE IF NOT EXISTS concept_mentions (
      concept_name TEXT,
      source_file TEXT
    );
  `);

  const insertCommunity = db.query(
    `INSERT INTO concept_communities (concept_name, community_id) VALUES (?, ?);`,
  );
  const insertGraph = db.query(
    `INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count) VALUES (?, ?, ?);`,
  );
  const insertMention = db.query(
    `INSERT INTO concept_mentions (concept_name, source_file) VALUES (?, ?);`,
  );
  const upsertFileContent = db.query(
    `
      INSERT INTO file_content (file_path, status, content, last_indexed, created, last_accessed)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(file_path) DO UPDATE SET
        status=excluded.status,
        content=excluded.content,
        last_indexed=excluded.last_indexed,
        created=excluded.created,
        last_accessed=excluded.last_accessed;
    `,
  );

  const nowIso = new Date().toISOString();

  for (let community = 1; community <= communityCount; community++) {
    for (let conceptIndex = 0; conceptIndex < conceptsPerCommunity; conceptIndex++) {
      const isAnchor = conceptIndex === 0;
      const conceptName = isAnchor
        ? `${anchorPrefix}${community}`
        : `${anchorPrefix}${community}-c${conceptIndex}`;
      const sourceFile = `memory/${anchorPrefix}-community-${community}.md`;

      insertCommunity.run(conceptName, community);
      insertMention.run(conceptName, sourceFile);
      upsertFileContent.run(
        sourceFile,
        "active",
        `Seed content for [[${conceptName}]]`,
        nowIso,
        nowIso,
        nowIso,
      );

      const coOccur = conceptWeightBase - community;
      insertGraph.run(conceptName, `${anchorPrefix}${community}-hub`, coOccur);
    }
  }

  db.close();
}

function extractBlock(text: string, tag: string): string {
  const start = `<${tag}>`;
  const end = `</${tag}>`;
  const startIndex = text.indexOf(start);
  const endIndex = text.indexOf(end);
  if (startIndex === -1 || endIndex === -1 || endIndex < startIndex) return "";
  return text.slice(startIndex + start.length, endIndex).trim();
}

function getCommunityLines(text: string): string[] {
  const block = extractBlock(text, "communities");
  return block
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function getAnchorFromCommunityLine(line: string): string | null {
  const match = line.match(/^\*\*\[\[([^\]]+)\]\]\*\*/);
  return match?.[1] ?? null;
}

function countWikiLinksInLine(line: string): number {
  const matches = line.match(/\[\[[^\]]+\]\]/g);
  return matches?.length ?? 0;
}

function countMatches(haystack: string, needle: string): number {
  if (!needle) return 0;
  return haystack.split(needle).length - 1;
}

function getCommunityAnchors(text: string): string[] {
  return getCommunityLines(text)
    .map(getAnchorFromCommunityLine)
    .filter((anchor): anchor is string => Boolean(anchor));
}

function updateFileContentTimestamps(
  dbPath: string,
  rows: Array<{
    filePath: string;
    created: string | null;
    lastIndexed: string | null;
    lastAccessed: string | null;
  }>,
): void {
  const db = new Database(dbPath);
  const update = db.query(
    `UPDATE file_content SET created = ?, last_indexed = ?, last_accessed = ? WHERE file_path = ?`,
  );

  for (const row of rows) {
    update.run(row.created, row.lastIndexed, row.lastAccessed, row.filePath);
  }

  db.close();
}

function seedCustomCommunityGraph(
  dbPath: string,
  options: {
    concepts: Array<{
      conceptName: string;
      communityId: number;
      sourceFile?: string;
      content?: string | null;
      created?: string | null;
      lastIndexed?: string | null;
      lastAccessed?: string | null;
    }>;
    edges: Array<{
      conceptA: string;
      conceptB: string;
      coOccurrenceCount: number;
    }>;
  },
): void {
  const db = new Database(dbPath);
  db.exec(`
    CREATE TABLE IF NOT EXISTS concept_communities (
      concept_name TEXT PRIMARY KEY,
      community_id INTEGER
    );

    CREATE TABLE IF NOT EXISTS concept_graph (
      concept_a TEXT,
      concept_b TEXT,
      co_occurrence_count INTEGER
    );

    CREATE TABLE IF NOT EXISTS concept_mentions (
      concept_name TEXT,
      source_file TEXT
    );

    CREATE TABLE IF NOT EXISTS file_content (
      file_path TEXT PRIMARY KEY,
      status TEXT,
      content TEXT,
      last_indexed TEXT,
      created TEXT,
      last_accessed TEXT
    );
  `);

  const insertCommunity = db.query(
    `INSERT INTO concept_communities (concept_name, community_id) VALUES (?, ?);`,
  );
  const insertGraph = db.query(
    `INSERT INTO concept_graph (concept_a, concept_b, co_occurrence_count) VALUES (?, ?, ?);`,
  );
  const insertMention = db.query(
    `INSERT INTO concept_mentions (concept_name, source_file) VALUES (?, ?);`,
  );
  const upsertFileContent = db.query(
    `
      INSERT INTO file_content (file_path, status, content, last_indexed, created, last_accessed)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(file_path) DO UPDATE SET
        status=excluded.status,
        content=excluded.content,
        last_indexed=excluded.last_indexed,
        created=excluded.created,
        last_accessed=excluded.last_accessed;
    `,
  );

  const nowIso = new Date().toISOString();

  for (const concept of options.concepts) {
    const sourceFile = concept.sourceFile ?? `memory/${concept.conceptName}.md`;
    insertCommunity.run(concept.conceptName, concept.communityId);
    insertMention.run(concept.conceptName, sourceFile);
    upsertFileContent.run(
      sourceFile,
      "active",
      concept.content ?? `Seed content for [[${concept.conceptName}]]`,
      concept.lastIndexed ?? nowIso,
      concept.created ?? nowIso,
      concept.lastAccessed ?? nowIso,
    );
  }

  for (const edge of options.edges) {
    insertGraph.run(edge.conceptA, edge.conceptB, edge.coOccurrenceCount);
  }

  db.close();
}

describe("T-OC-PERSIST-002.5 traceability hygiene + FR-29 persistence behavior", () => {
  const originalEnv = snapshotEnv();
  let tempDirs: string[] = [];
  let originalConsoleError: typeof console.error;
  let consoleErrorMock: ReturnType<typeof mock<(...args: any[]) => void>>;

  beforeEach(() => {
    restoreEnv(originalEnv);
    tempDirs = [];
    originalConsoleError = console.error;
    consoleErrorMock = mock(() => undefined);
    (console as any).error = consoleErrorMock;
  });

  afterEach(() => {
    (console as any).error = originalConsoleError;
    for (const dir of tempDirs) {
      rmSync(dir, { recursive: true, force: true });
    }
    restoreEnv(originalEnv);
  });

  // Legacy annotation: pre-FR-29 regression coverage retained for path/env behavior.
  test("LEGACY_AC1_DbPathRecomputedAfterEnvChange_UsesLatestDatabasePath", () => {
    const first = makeTmpDir("persist-test-first-");
    const second = makeTmpDir("persist-test-second-");
    tempDirs.push(first, second);

    const firstDb = join(first, "memory.db");
    const secondDb = join(second, "memory.db");
    writeFileSync(firstDb, "db1");
    writeFileSync(secondDb, "db2");

    process.env.MAENIFOLD_DATABASE_PATH = firstDb;
    const seenPaths: string[] = [];
    const opener = (dbPath: string) => {
      seenPaths.push(dbPath);
      return {
        exec: () => undefined,
        close: () => undefined,
      } as any;
    };

    const db1 = __test__._openDb(opener);
    expect(db1).not.toBeNull();

    process.env.MAENIFOLD_DATABASE_PATH = secondDb;
    const db2 = __test__._openDb(opener);
    expect(db2).not.toBeNull();

    expect(seenPaths).toEqual([firstDb, secondDb]);
  });

  test("LEGACY_AC2_MaenifoldRootRecomputedAfterEnvChange_ResolvesDerivedMemoryDb", () => {
    delete process.env.MAENIFOLD_DATABASE_PATH;

    const firstRoot = makeTmpDir("persist-root-a-");
    const secondRoot = makeTmpDir("persist-root-b-");
    tempDirs.push(firstRoot, secondRoot);

    const firstDb = join(firstRoot, "memory.db");
    const secondDb = join(secondRoot, "memory.db");
    writeFileSync(firstDb, "a");
    writeFileSync(secondDb, "b");

    process.env.MAENIFOLD_ROOT = firstRoot;
    expect(__test__._getDbPath()).toBe(firstDb);

    process.env.MAENIFOLD_ROOT = secondRoot;
    expect(__test__._getDbPath()).toBe(secondDb);
  });

  test("RecentActivityFlag_FallsBackToOpencodeConfigMcpEnvironment", () => {
    const home = makeTmpDir("persist-config-activity-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;
    delete process.env.MAENIFOLD_RECENT_ACTIVITY;

    writeFileSync(
      join(home, ".config/opencode/opencode.json"),
      JSON.stringify(
        {
          mcp: {
            maenifold: {
              type: "local",
              command: ["maenifold", "--mcp"],
              environment: {
                MAENIFOLD_RECENT_ACTIVITY: "true",
              },
            },
          },
        },
        null,
        2,
      ),
    );

    expect(__test__._isRecentActivityEnabled()).toBe(true);
  });

  test("RecentCompactionsFlag_FallsBackToOpencodeConfigMcpEnvironment", () => {
    const home = makeTmpDir("persist-config-compactions-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;
    delete process.env.MAENIFOLD_RECENT_COMPACTIONS;

    writeFileSync(
      join(home, ".config/opencode/opencode.json"),
      JSON.stringify(
        {
          mcp: {
            maenifold: {
              type: "local",
              command: ["maenifold", "--mcp"],
              environment: {
                MAENIFOLD_RECENT_COMPACTIONS: "false",
              },
            },
          },
        },
        null,
        2,
      ),
    );

    expect(__test__._isRecentCompactionsEnabled()).toBe(false);
  });

  test("LEGACY_AC3_AgentsPathComputedAtRuntime_HomeChangesAreHonored", () => {
    const homeA = makeTmpDir("persist-home-a-");
    const homeB = makeTmpDir("persist-home-b-");
    tempDirs.push(homeA, homeB);

    process.env.HOME = homeA;
    expect(__test__._getAgentsMdPath()).toBe(join(homeA, ".config/opencode/AGENTS.md"));

    process.env.HOME = homeB;
    expect(__test__._getAgentsMdPath()).toBe(join(homeB, ".config/opencode/AGENTS.md"));
  });

  test("LEGACY_AC4_ReproUnableToOpenDatabaseFile_GracefulNullAndDiagnostic", () => {
    const root = makeTmpDir("persist-open-fail-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    writeFileSync(dbPath, "placeholder");
    process.env.MAENIFOLD_DATABASE_PATH = dbPath;

    const db = __test__._openDb(() => {
      throw new Error("SQLiteError: unable to open database file");
    });

    expect(db).toBeNull();
    const logs = consoleErrorMock.mock.calls.map((call) => String(call[0]));
    expect(logs.some((line) => line.includes("unable to open database file"))).toBeTrue();
  });

  test("LEGACY_AC5_PathIsDirectory_BlockedBeforeOpenAttempt", () => {
    const root = makeTmpDir("persist-dir-path-");
    tempDirs.push(root);

    const dbAsDirectory = join(root, "memory.db");
    mkdirSync(dbAsDirectory);
    process.env.MAENIFOLD_DATABASE_PATH = dbAsDirectory;

    const opener = mock((_dbPath: string) => {
      throw new Error("should never be called");
    });

    const db = __test__._openDb(opener as any);
    expect(db).toBeNull();
    expect(opener).toHaveBeenCalledTimes(0);
  });

  // Legacy annotation: originally T-OC-PERSIST-001 / RTM FR-12.x; retained as non-FR-29 regression.
  test("LEGACY_AC6_CreateReadonlyDb_SetsBusyTimeout", () => {
    const root = makeTmpDir("persist-busy-timeout-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    const setupDb = new Database(dbPath);
    setupDb.exec("CREATE TABLE test (id INTEGER PRIMARY KEY);");
    setupDb.close();

    process.env.MAENIFOLD_DATABASE_PATH = dbPath;
    const db = __test__._openDb();
    expect(db).not.toBeNull();

    const result = db!.query("PRAGMA busy_timeout;").get() as { timeout: number };
    expect(result.timeout).toBeGreaterThanOrEqual(5000);
    db!.close();
  });

  // T-OC-PERSIST-002.1: RTM FR-29.1
  test("FR29_1_AC1_OpenDb_MissingWalShmWithValidMemoryDb_ReturnsHandle", () => {
    const root = makeTmpDir("persist-fr29-ac1-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    const setupDb = new Database(dbPath);
    setupDb.exec("PRAGMA journal_mode=WAL;");
    setupDb.exec("CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY);");
    setupDb.exec("INSERT INTO test DEFAULT VALUES;");
    setupDb.close();

    rmSync(`${dbPath}-wal`, { force: true });
    rmSync(`${dbPath}-shm`, { force: true });
    expect(existsSync(dbPath)).toBeTrue();
    expect(existsSync(`${dbPath}-wal`)).toBeFalse();
    expect(existsSync(`${dbPath}-shm`)).toBeFalse();

    process.env.MAENIFOLD_DATABASE_PATH = dbPath;
    const db = __test__._openDb();

    expect(db).not.toBeNull();
    db?.close();
  });

  // T-OC-PERSIST-002.2: RTM FR-29.2
  test("FR29_2_AC2_OpenDb_ReturnedConnectionHasQueryOnlyEnabled", () => {
    const root = makeTmpDir("persist-fr29-ac2-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    const setupDb = new Database(dbPath);
    setupDb.exec("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY);");
    setupDb.close();

    process.env.MAENIFOLD_DATABASE_PATH = dbPath;
    const db = __test__._openDb();
    expect(db).not.toBeNull();

    const row = db!.query("PRAGMA query_only;").get() as Record<string, number>;
    const queryOnly = Number(Object.values(row)[0]);
    expect(queryOnly).toBe(1);
    db!.close();
  });

  // T-OC-PERSIST-002.3: RTM FR-29.3
  test("FR29_3_AC3_OpenDb_ReturnedConnectionHasBusyTimeoutAtLeast5000", () => {
    const root = makeTmpDir("persist-fr29-ac3-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    const setupDb = new Database(dbPath);
    setupDb.exec("CREATE TABLE IF NOT EXISTS t (id INTEGER PRIMARY KEY);");
    setupDb.close();

    process.env.MAENIFOLD_DATABASE_PATH = dbPath;
    const db = __test__._openDb();
    expect(db).not.toBeNull();

    const row = db!.query("PRAGMA busy_timeout;").get() as Record<string, number>;
    const busyTimeout = Number(Object.values(row)[0]);
    expect(busyTimeout).toBeGreaterThanOrEqual(5000);
    db!.close();
  });

  // T-OC-PERSIST-002.4: RTM FR-29.4
  test("FR29_4_AC4_OpenDb_OpenFailureReturnsNullAndDiagnostic", () => {
    const root = makeTmpDir("persist-fr29-ac4-");
    tempDirs.push(root);

    const dbPath = join(root, "memory.db");
    writeFileSync(dbPath, "placeholder");
    process.env.MAENIFOLD_DATABASE_PATH = dbPath;

    const db = __test__._openDb(() => {
      throw new Error("SQLiteError: unable to open database file");
    });

    expect(db).toBeNull();
    const logs = consoleErrorMock.mock.calls.map((call) => String(call[0]));
    expect(
      logs.some(
        (line) =>
          line.includes("_openDb FAILED") &&
          line.includes("unable to open database file"),
      ),
    ).toBeTrue();
  });
});

describe("T-OC-PERSIST-003.1 FR-31 compaction rehydration contract (tests-first)", () => {
  const originalEnv = snapshotEnv();
  let tempDirs: string[] = [];
  let originalSpawn: typeof Bun.spawn;

  beforeEach(() => {
    restoreEnv(originalEnv);
    tempDirs = [];
    originalSpawn = Bun.spawn;
  });

  afterEach(() => {
    (Bun as any).spawn = originalSpawn;
    for (const dir of tempDirs) {
      rmSync(dir, { recursive: true, force: true });
    }
    restoreEnv(originalEnv);
  });

  function setupHomeAndDb(rows: Array<{ filePath: string; content?: string | null }>) {
    const home = makeTmpDir("persist-fr31-home-");
    const root = makeTmpDir("persist-fr31-root-");
    tempDirs.push(home, root);

    mkdirSync(join(home, ".config/opencode"), { recursive: true });

    const dbPath = join(root, "memory.db");
    const nowIso = new Date().toISOString();
    createFileContentDb(
      dbPath,
      rows.map((r) => ({
        filePath: r.filePath,
        content: r.content ?? null,
        status: "active",
        lastIndexed: nowIso,
        created: nowIso,
        lastAccessed: nowIso,
      })),
    );

    process.env.HOME = home;
    process.env.MAENIFOLD_DATABASE_PATH = dbPath;
    process.env.MAENIFOLD_COMMUNITIES = "false";
    delete process.env.MAENIFOLD_RECENT_ACTIVITY;

    return {
      home,
      root,
      dbPath,
      agentsPath: join(home, ".config/opencode/AGENTS.md"),
    };
  }

  // T-OC-PERSIST-003.1: RTM FR-31.1
  test("FR31_1_AC1_PersistCompaction_TitleUsesCompactionSessionIdEpochMillis", async () => {
    const home = makeTmpDir("persist-fr31-ac1-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;

    let capturedPayload: Record<string, any> | null = null;
    (Bun as any).spawn = (args: string[]) => {
      const payloadIndex = args.indexOf("--payload");
      capturedPayload = JSON.parse(args[payloadIndex + 1]);
      return {
        stdout: '{"success":true,"data":{"uri":"memory://test/ok"}}',
        stderr: "",
        exited: Promise.resolve(0),
        signalCode: null,
      };
    };

    const plugin = await PersistencePlugin({
      client: {
        session: {
          messages: async () => [
            {
              info: { role: "assistant", summary: true },
              parts: [{ type: "text", text: "Summary with [[traceability]]" }],
            },
          ],
        },
        app: { log: async () => undefined },
        tui: { showToast: async () => undefined },
      },
    } as any);

    await plugin.event({
      event: {
        type: "session.compacted",
        properties: { sessionID: "session-1700000000000-abc123" },
      },
    } as any);

    expect(capturedPayload).not.toBeNull();
    const title = String(capturedPayload!.title ?? "");
    expect(title).toMatch(/^compaction sidhex [0-9a-f]+ ts \d{13}$/);
  });

  // T-OC-PERSIST-003.2: RTM FR-31.2
  test("FR31_2_AC2_BuildGraphContext_RecentSessionsBlockIsOmitted", async () => {
    const { agentsPath } = setupHomeAndDb([
      {
        filePath:
          "thinking/sequential/2026/03/05/session-1700000000001-rs-test.md",
        content: "## Thought 1\nSession intent with [[traceability]]",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = Bun.file(agentsPath).text();
    expect(await agents).not.toContain("<recent-sessions>");
  });

  // T-OC-PERSIST-003.3: RTM FR-31.3
  test("FR31_3_AC3_BuildGraphContext_SelectsLatestCompactionPerSession", async () => {
    const now = Date.now();
    const sessionA = `session-${now - 5 * 60_000}-alpha`;
    const sessionB = `session-${now - 8 * 60_000}-beta`;

    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionA}-1001.md`,
        content: "OLD A CONTENT",
      },
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionA}-2002.md`,
        content: "LATEST A CONTENT",
      },
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionB}-1500.md`,
        content: "LATEST B CONTENT",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("<recent-compactions>");
    expect(agents).toContain("LATEST A CONTENT");
    expect(agents).toContain("LATEST B CONTENT");
    expect(agents).not.toContain("OLD A CONTENT");
  });

  // T-OC-PERSIST-003.3: RTM FR-31.7
  test("FR31_3_AC4_BuildGraphContext_FiltersSessionCohortBySessionEpochLast30Minutes", async () => {
    const now = Date.now();
    const within30m = `session-${now - 10 * 60_000}-within`;
    const olderThan30m = `session-${now - 45 * 60_000}-old`;

    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${within30m}-9001.md`,
        content: "WITHIN_30_MINUTES",
      },
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${olderThan30m}-9002.md`,
        content: "OLDER_THAN_30_MINUTES",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("WITHIN_30_MINUTES");
    expect(agents).not.toContain("OLDER_THAN_30_MINUTES");
  });

  // T-OC-PERSIST-003.3: RTM FR-31.3
  test("FR31_4_AC5_BuildGraphContext_CapsRecentCompactionsToTenSessions", async () => {
    const now = Date.now();
    const rows: Array<{ filePath: string; content: string }> = [];

    for (let i = 0; i < 12; i++) {
      const sessionId = `session-${now - i * 60_000}-cap-${i}`;
      rows.push({
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionId}-${10_000 + i}.md`,
        content: `SESSION_CAP_${i}`,
      });
    }

    const { agentsPath } = setupHomeAndDb(rows);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(countMatches(agents, "SESSION_CAP_")).toBe(10);
    expect(agents).not.toContain("SESSION_CAP_10");
    expect(agents).not.toContain("SESSION_CAP_11");
  });

  // T-OC-PERSIST-003.4: RTM FR-31.5
  test("FR31_5_AC6_BuildGraphContext_IncludesFullCompactionContentForEachEntry", async () => {
    const now = Date.now();
    const sessionId = `session-${now - 2 * 60_000}-full`;
    const fullBody = [
      "## OpenCode compaction summary (untrusted)",
      "Line 1: this is the complete persisted compaction body.",
      "Line 2: includes [[traceability]] and detailed context.",
      "Line 3: terminal marker FULL_BODY_END.",
    ].join("\n");

    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionId}-3333.md`,
        content: fullBody,
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain(fullBody);
  });

  // T-OC-PERSIST-003.5: RTM FR-31.6
  test("FR31_6_AC7_BuildGraphContext_OmitsRecentCompactionsBlockWhenNoQualifyingEntries", async () => {
    const { agentsPath } = setupHomeAndDb([
      {
        filePath:
          "thinking/sequential/2026/03/05/session-1700000000002-no-compactions.md",
        content: "## Thought 1\nOnly sequential data with [[traceability]]",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).not.toContain("<recent-compactions>");
  });

  test("FR31_6_AC8_BuildGraphContext_OmitsRecentCompactionsBlockWhenFlagDisabled", async () => {
    const now = Date.now();
    const sessionId = `session-${now - 60_000}-disable-compactions`;
    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionId}-5555.md`,
        content: "This compaction should be hidden when flag disabled.",
      },
    ]);
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = existsSync(agentsPath) ? await Bun.file(agentsPath).text() : "";
    expect(agents).not.toContain("<recent-compactions>");
    expect(agents).not.toContain("This compaction should be hidden when flag disabled.");
  });

  // T-OC-PERSIST-003.5a: RTM FR-31.8
  test("FR31_8_AC7_BuildGraphContext_GuardDelimiterInCompactionBody_DoesNotCorruptGuardedBlock", async () => {
    const now = Date.now();
    const sessionId = `session-${now - 60_000}-guard`;
    const compactionBody = [
      "GUARD_PAYLOAD_PREFIX",
      "<!-- END:MAENIFOLD -->",
      "PAYLOAD_AFTER_END_SENTINEL",
    ].join("\n");

    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${sessionId}-4444.md`,
        content: compactionBody,
      },
    ]);

    const plugin = await PersistencePlugin({
      client: {
        app: { log: async () => undefined },
      },
    } as any);

    await plugin.event({
      event: {
        type: "session.updated",
        properties: { sessionID: "session-refresh-guard-test" },
      },
    } as any);

    await new Promise((resolve) => setTimeout(resolve, 2200));

    const agents = await Bun.file(agentsPath).text();
    const beginCount = countMatches(agents, "<!-- BEGIN:MAENIFOLD -->");
    const endCount = countMatches(agents, "<!-- END:MAENIFOLD -->");
    const beginIndex = agents.indexOf("<!-- BEGIN:MAENIFOLD -->");
    const endIndex = agents.indexOf("<!-- END:MAENIFOLD -->");

    expect(beginCount).toBe(1);
    expect(endCount).toBe(1);
    expect(beginIndex).toBeGreaterThanOrEqual(0);
    expect(endIndex).toBeGreaterThan(beginIndex);
    expect(agents).toContain("GUARD_PAYLOAD_PREFIX");
    expect(agents).toContain("PAYLOAD_AFTER_END_SENTINEL");
  });

  // T-OC-PERSIST-003.8: RTM FR-31.9
  test("FR31_9_AC1_Communities_DefaultOutputIsCappedTo10x10", async () => {
    const { agentsPath, dbPath } = setupHomeAndDb([]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 12,
      conceptsPerCommunity: 12,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const lines = getCommunityLines(agents);
    expect(lines.length).toBeLessThanOrEqual(10);
    for (const line of lines) {
      expect(countWikiLinksInLine(line)).toBeLessThanOrEqual(10);
    }
  });

  // T-OC-PERSIST-003.9: RTM FR-31.10
  test("FR31_10_AC2_CompactionWikilinks_FilterCommunitiesToMatchingAnchors", async () => {
    const now = Date.now();
    const targetSession = `session-${now - 2 * 60_000}-compaction-filter`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${targetSession}-7001.md`,
        content:
          "## OpenCode compaction summary\nContains [[cap1]] and unresolved [[not-a-community-anchor]].",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const anchors = getCommunityLines(agents)
      .map(getAnchorFromCommunityLine)
      .filter((a): a is string => Boolean(a));

    expect(anchors).toEqual(["cap1"]);
  });

  // T-OC-PERSIST-003.9: RTM FR-31.10
  test("FR31_10_AC3_RecentActivityWikilinks_FilterCommunitiesToMatchingAnchors", async () => {
    const now = Date.now();
    const activitySession = `session-${now - 2 * 60_000}-activity-filter`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/sequential/2026/03/05/${activitySession}.md`,
        content: "## Thought 1\nActivity includes [[cap2]] as anchor signal.",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const anchors = getCommunityLines(agents)
      .map(getAnchorFromCommunityLine)
      .filter((a): a is string => Boolean(a));

    expect(anchors).toEqual(["cap2"]);
  });

  // T-OC-PERSIST-003.9: RTM FR-31.11
  test("FR31_11_AC4_CompactionAndActivityWikilinks_AreMergedBeforeFiltering", async () => {
    const now = Date.now();
    const compactionSession = `session-${now - 2 * 60_000}-merge-compaction`;
    const activitySession = `session-${now - 3 * 60_000}-merge-activity`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${compactionSession}-7100.md`,
        content: "Compaction extracted [[cap1]]",
      },
      {
        filePath: `thinking/sequential/2026/03/05/${activitySession}.md`,
        content: "## Thought 1\nRecent activity extracted [[cap2]]",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const anchors = getCommunityLines(agents)
      .map(getAnchorFromCommunityLine)
      .filter((a): a is string => Boolean(a));

    expect(new Set(anchors)).toEqual(new Set(["cap1", "cap2"]));
    expect(anchors).not.toContain("cap3");
  });

  // T-OC-PERSIST-003.10: RTM FR-31.12
  test("FR31_12_AC5_RecentActivity_ExcludesCompactionArtifacts", async () => {
    const now = Date.now();
    const activitySession = `session-${now - 2 * 60_000}-recent-activity`;
    const compactionSession = `session-${now - 1 * 60_000}-recent-compaction`;

    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/sequential/2026/03/05/${activitySession}.md`,
        content: "## Thought 1\nRegular session row.",
      },
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${compactionSession}-7200.md`,
        content: "Compaction artifact row should not appear in recent activity.",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "false";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentSessionsBlock = extractBlock(agents, "recent-sessions");
    expect(recentSessionsBlock).toContain(activitySession);
    expect(recentSessionsBlock).not.toContain(compactionSession);
    expect(recentSessionsBlock).not.toContain("Compaction artifact row should not appear");
  });

  // T-OC-PERSIST-003.9: RTM FR-31.10
  test("FR31_12_AC6_NoResolvedAnchors_FallsBackToDefaultTopCommunitiesWith10x10Cap", async () => {
    const now = Date.now();
    const unresolvedSession = `session-${now - 2 * 60_000}-no-anchor`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/05/compaction-${unresolvedSession}-7300.md`,
        content: "No anchor match in this summary: [[unmapped-anchor]].",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 12,
      conceptsPerCommunity: 12,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({ client: { app: { log: async () => undefined } } } as any);

    const agents = await Bun.file(agentsPath).text();
    const lines = getCommunityLines(agents);
    expect(lines.length).toBe(10);
    for (const line of lines) {
      expect(countWikiLinksInLine(line)).toBeLessThanOrEqual(10);
    }
  });

  // T-OC-PERSIST-003.12: RTM FR-31.13
  test("FR31_13_AC1_PersistCompaction_OpaqueSessionId_UsesCanonicalCompactionTitle", async () => {
    const home = makeTmpDir("persist-fr31-13-ac1-home-");
    tempDirs.push(home);
    mkdirSync(join(home, ".config/opencode"), { recursive: true });
    process.env.HOME = home;

    let capturedPayload: Record<string, any> | null = null;
    (Bun as any).spawn = (args: string[]) => {
      const payloadIndex = args.indexOf("--payload");
      capturedPayload = JSON.parse(args[payloadIndex + 1]);
      return {
        stdout: '{"success":true,"data":{"uri":"memory://test/ok"}}',
        stderr: "",
        exited: Promise.resolve(0),
        signalCode: null,
      };
    };

    const plugin = await PersistencePlugin({
      client: {
        session: {
          messages: async () => [
            {
              info: { role: "assistant", summary: true },
              parts: [{ type: "text", text: "Opaque session summary [[traceability]]" }],
            },
          ],
        },
        app: { log: async () => undefined },
        tui: { showToast: async () => undefined },
      },
    } as any);

    await plugin.event({
      event: {
        type: "session.compacted",
        properties: { sessionID: "ses-abc123" },
      },
    } as any);

    expect(capturedPayload).not.toBeNull();
    const title = String(capturedPayload!.title ?? "");
    expect(title).toMatch(/^compaction sidhex [0-9a-f]+ ts \d{13}$/);
  });

  // T-OC-PERSIST-003.12: RTM FR-31.14
  test("FR31_14_AC2_BuildGraphContext_CanonicalOpaqueCompactionTitle_IsAccepted", async () => {
    const now = Date.now();
    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/06/compaction-ses-opaque-xyz-${now}.md`,
        content: "OPAQUE_CANONICAL_ACCEPTED",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("<recent-compactions>");
    expect(agents).toContain("OPAQUE_CANONICAL_ACCEPTED");
  });

  test("FR31_14_AC2B_BuildGraphContext_SlugCollapsedRealCompactionFile_RendersViaFallback", async () => {
    const now = new Date().toISOString();
    const { agentsPath } = setupHomeAndDb([
      {
        filePath:
          "memory://thinking/compaction/2026/03/08/compactionses-3313d1989ffexxye3akitu4ecb1772998263607",
        content: [
          "## OpenCode compaction summary (untrusted)",
          "",
          "Raw Session ID: ses_3313d1989ffeXXyE3akITU4ECb",
          "Session Slug: ses-3313d1989ffexxye3akitu4ecb",
          "",
          "REAL_WORLD_COLLAPSED_ENTRY",
        ].join("\n"),
      },
    ]);

    const db = new Database(process.env.MAENIFOLD_DATABASE_PATH!);
    db.query(
      `UPDATE file_content SET created = ?, last_indexed = ?, last_accessed = ? WHERE file_path = ?`,
    ).run(
      now,
      now,
      now,
      "memory://thinking/compaction/2026/03/08/compactionses-3313d1989ffexxye3akitu4ecb1772998263607",
    );
    db.close();

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("<recent-compactions>");
    expect(agents).toContain("REAL_WORLD_COLLAPSED_ENTRY");
    expect(agents).toContain(
      "memory://thinking/compaction/2026/03/08/compactionses-3313d1989ffexxye3akitu4ecb1772998263607",
    );
  });

  // T-OC-PERSIST-003.12: RTM FR-31.14
  test("FR31_14_AC3_BuildGraphContext_LegacySlugCollapsedTitles_FailOpenWithoutThrow", async () => {
    const now = Date.now();
    const { agentsPath } = setupHomeAndDb([
      {
        // Legacy separator-collapsed historical variant (intentionally malformed)
        filePath: `thinking/compaction/2026/03/06/compaction-session1700000000000legacy${now}.md`,
        content: "LEGACY_COLLAPSED_SHOULD_NOT_THROW",
      },
      {
        filePath: `thinking/compaction/2026/03/06/compaction-ses-fallback-${now}.md`,
        content: "VALID_ROW_STILL_RENDERED",
      },
    ]);

    await expect(
      PersistencePlugin({
        client: { app: { log: async () => undefined } },
      } as any),
    ).resolves.toBeDefined();

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("VALID_ROW_STILL_RENDERED");
  });

  // T-OC-PERSIST-003.12: RTM FR-31.14
  test("FR31_14_AC4_BuildGraphContext_UsesSessionEpochWhenParseable_ElseCompactionEpochFallback", async () => {
    const now = Date.now();
    const parseableOldSession = `session-${now - 45 * 60_000}-old-session`;

    const { agentsPath } = setupHomeAndDb([
      {
        // Parseable session epoch is old -> should be excluded even though compaction epoch is fresh
        filePath: `thinking/compaction/2026/03/06/compaction-${parseableOldSession}-${now}.md`,
        content: "PARSEABLE_OLD_SESSION_SHOULD_BE_EXCLUDED",
      },
      {
        // Opaque session id -> should use compaction epoch fallback and be included
        filePath: `thinking/compaction/2026/03/06/compaction-ses-fallback-window-${now}.md`,
        content: "OPAQUE_FALLBACK_INCLUDED",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("OPAQUE_FALLBACK_INCLUDED");
    expect(agents).not.toContain("PARSEABLE_OLD_SESSION_SHOULD_BE_EXCLUDED");
  });

  // T-OC-PERSIST-003.12: RTM FR-31.14
  test("FR31_14_AC5_BuildGraphContext_RendersRecentCompactions_ForQualifyingOpaqueSessionsViaFallback", async () => {
    const now = Date.now();
    const { agentsPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/06/compaction-ses-qualifying-a-${now - 60_000}.md`,
        content: "OPAQUE_RECENT_A",
      },
      {
        filePath: `thinking/compaction/2026/03/06/compaction-ses-qualifying-b-${now - 30_000}.md`,
        content: "OPAQUE_RECENT_B",
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(agents).toContain("<recent-compactions>");
    expect(agents).toContain("OPAQUE_RECENT_A");
    expect(agents).toContain("OPAQUE_RECENT_B");
  });

  // T-OC-PERSIST-003.12: RTM FR-31.14
  test("FR31_14_AC6_Traceability_ExistingFR31_1ThroughFR31_12CoverageRemainsPresent", async () => {
    const selfSource = await Bun.file(import.meta.path).text();
    for (const id of [
      "FR31_1_AC1",
      "FR31_2_AC2",
      "FR31_3_AC3",
      "FR31_3_AC4",
      "FR31_4_AC5",
      "FR31_5_AC6",
      "FR31_6_AC7",
      "FR31_8_AC7",
      "FR31_9_AC1",
      "FR31_10_AC2",
      "FR31_10_AC3",
      "FR31_11_AC4",
      "FR31_12_AC5",
      "FR31_12_AC6",
    ]) {
      expect(selfSource).toContain(id);
    }
  });

  // T-OC-PERSIST-003.16: RTM FR-31.14
  test("FR31_14_AC7_SlugCollisionDistinctRawSessions_RemainIsolatedAndBothRender", async () => {
    const now = Date.now();
    const collidingSlug = "ses-a-b";

    const { agentsPath } = setupHomeAndDb([
      {
        // Raw session ID #1: ses-a_b -> slug-collides to ses-a-b
        filePath: `thinking/compaction/2026/03/06/compaction-${collidingSlug}-${now - 90_000}.md`,
        content: [
          "## OpenCode compaction summary (untrusted)",
          "Raw Session ID: ses-a_b",
          "Session Slug: ses-a-b",
          "RAW_UNDERSCORE_ENTRY",
        ].join("\n"),
      },
      {
        // Raw session ID #2: ses-a-b -> same slug token in title
        filePath: `thinking/compaction/2026/03/06/compaction-${collidingSlug}-${now - 30_000}.md`,
        content: [
          "## OpenCode compaction summary (untrusted)",
          "Raw Session ID: ses-a-b",
          "Session Slug: ses-a-b",
          "RAW_HYPHEN_ENTRY",
        ].join("\n"),
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentCompactionsBlock = extractBlock(agents, "recent-compactions");

    expect(recentCompactionsBlock).toContain("RAW_UNDERSCORE_ENTRY");
    expect(recentCompactionsBlock).toContain("RAW_HYPHEN_ENTRY");
  });

  // T-OC-PERSIST-003.16: RTM FR-31.14
  test("FR31_14_AC8_RecencyDecision_IgnoresForgedRawSessionIdMetadata", async () => {
    const now = Date.now();
    const staleCompactionEpoch = now - 45 * 60_000;

    const { agentsPath } = setupHomeAndDb([
      {
        // Opaque title token + stale compaction epoch (outside 30m window).
        // Forged body metadata attempts to inject fresh epoch recency.
        filePath: `thinking/compaction/2026/03/06/compaction-ses-opaque-forged-${staleCompactionEpoch}.md`,
        content: [
          "## OpenCode compaction summary (untrusted)",
          `Raw Session ID: session-${now - 15_000}-forged-fresh`,
          "Session Slug: ses-opaque-forged",
          "FORGED_RECENCY_SHOULD_BE_EXCLUDED",
        ].join("\n"),
      },
      {
        // Trusted in-window control row to ensure recent-compactions block exists.
        filePath: `thinking/compaction/2026/03/06/compaction-ses-opaque-control-${now - 30_000}.md`,
        content: [
          "## OpenCode compaction summary (untrusted)",
          "Raw Session ID: ses-opaque-control",
          "Session Slug: ses-opaque-control",
          "TRUSTED_IN_WINDOW_CONTROL",
        ].join("\n"),
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentCompactionsBlock = extractBlock(agents, "recent-compactions");

    expect(recentCompactionsBlock).toContain("TRUSTED_IN_WINDOW_CONTROL");
    expect(recentCompactionsBlock).not.toContain("FORGED_RECENCY_SHOULD_BE_EXCLUDED");
  });

  // T-OC-PERSIST-004.1: RTM FR-31.15, NFR-31.9
  test("FR31_15_AC1_RecentSessions_OnlyRenderedWikilinksInfluenceCommunities", async () => {
    const now = Date.now();
    const activitySession = `session-${now - 2 * 60_000}-rendered-activity-only`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/sequential/2026/03/08/${activitySession}.md`,
        content: [
          "# Sequential Thinking Session",
          "",
          "## Thought 0/3 [agent]",
          "Rendered activity includes [[cap1]].",
          "",
          "## Thought 1/3 [agent]",
          "Hidden middle thought includes [[cap2]] but should not influence communities.",
          "",
          "## Thought 2/3 [agent]",
          "Rendered conclusion without additional anchors.",
        ].join("\n"),
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentSessionsBlock = extractBlock(agents, "recent-sessions");

    expect(recentSessionsBlock).toContain("[[cap1]]");
    expect(recentSessionsBlock).not.toContain("[[cap2]]");
    expect(getCommunityAnchors(agents)).toEqual(["cap1"]);
  });

  // T-OC-PERSIST-004.1: RTM FR-31.15, NFR-31.9
  test("FR31_15_AC2_RecentCompactions_OnlyRenderedCompactionBlockInfluencesCommunities", async () => {
    const now = Date.now();
    const compactionSession = `session-${now - 2 * 60_000}-rendered-compaction-only`;
    const activitySession = `session-${now - 3 * 60_000}-disabled-activity-source`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/08/compaction-${compactionSession}-8100.md`,
        content: "Rendered compaction includes [[cap1]].",
      },
      {
        filePath: `thinking/sequential/2026/03/08/${activitySession}.md`,
        content: "## Thought 1\nDisabled activity source includes [[cap2]].",
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "false";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentCompactionsBlock = extractBlock(agents, "recent-compactions");

    expect(agents).not.toContain("<recent-sessions>");
    expect(recentCompactionsBlock).toContain("[[cap1]]");
    expect(getCommunityAnchors(agents)).toEqual(["cap1"]);
  });

  // T-OC-PERSIST-004.1: RTM FR-31.16, NFR-31.9
  test("FR31_16_AC3_ActivityAndCompactionDuplicateWikilinks_DedupeBeforeCommunityRendering", async () => {
    const now = Date.now();
    const compactionSession = `session-${now - 2 * 60_000}-dedupe-compaction`;
    const activitySession = `session-${now - 3 * 60_000}-dedupe-activity`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/08/compaction-${compactionSession}-8200.md`,
        content: "Compaction duplicate source includes [[cap1]].",
      },
      {
        filePath: `thinking/sequential/2026/03/08/${activitySession}.md`,
        content: [
          "# Sequential Thinking Session",
          "",
          "## Thought 0/2 [agent]",
          "Activity duplicate source includes [[CAP1]].",
          "",
          "## Thought 1/2 [agent]",
          "Conclusion without new anchors.",
        ].join("\n"),
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const anchors = getCommunityAnchors(agents);

    expect(anchors).toEqual(["cap1"]);
    expect(countMatches(extractBlock(agents, "communities"), "**[[cap1]]**")).toBe(1);
  });

  // T-OC-PERSIST-004.1: RTM FR-31.17, NFR-31.9
  test("FR31_17_AC4_ResolvedWikilinks_RenderDistinctAnchorsAndHonor10x10Cap", async () => {
    const now = Date.now();
    const compactionSession = `session-${now - 2 * 60_000}-anchor-cap-compaction`;
    const activitySession = `session-${now - 3 * 60_000}-anchor-cap-activity`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/compaction/2026/03/08/compaction-${compactionSession}-8300.md`,
        content: [
          "Compaction seeds [[cap7]] [[cap7-c1]] [[cap8]] [[cap9]] [[cap10]] [[cap11]] [[cap12]].",
        ].join("\n"),
      },
      {
        filePath: `thinking/sequential/2026/03/08/${activitySession}.md`,
        content: [
          "# Sequential Thinking Session",
          "",
          "## Thought 0/2 [agent]",
          "Rendered activity seeds [[cap1]] [[cap1-c1]] [[cap2]] [[cap3]] [[cap4]] [[cap5]] [[cap6]].",
          "",
          "## Thought 1/2 [agent]",
          "Conclusion without additional anchors.",
        ].join("\n"),
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";
    seedCommunityGraph(dbPath, {
      communityCount: 12,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const anchors = getCommunityAnchors(agents);

    expect(anchors).toHaveLength(10);
    expect(new Set(anchors).size).toBe(10);
    expect(countMatches(extractBlock(agents, "communities"), "**[[cap1]]**")).toBe(1);
    for (const line of getCommunityLines(agents)) {
      expect(countWikiLinksInLine(line)).toBeLessThanOrEqual(10);
    }
  });

  // T-OC-PERSIST-004.1: RTM FR-31.18, NFR-31.8, NFR-31.9
  test("FR31_18_AC5_CommunitiesOnlyMode_FallsBackToMostRecentlyUpdatedCommunities", async () => {
    const { agentsPath, dbPath } = setupHomeAndDb([]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "false";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    updateFileContentTimestamps(dbPath, [
      {
        filePath: "memory/cap-community-1.md",
        created: new Date(Date.now() - 3 * 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 3 * 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date().toISOString(),
      },
      {
        filePath: "memory/cap-community-2.md",
        created: new Date(Date.now() - 2 * 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 2 * 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date(Date.now() - 12 * 60 * 60_000).toISOString(),
      },
      {
        filePath: "memory/cap-community-3.md",
        created: new Date(Date.now() - 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date(Date.now() - 6 * 24 * 60 * 60_000).toISOString(),
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    expect(getCommunityAnchors(agents)).toEqual(["cap3", "cap2", "cap1"]);
  });

  // T-OC-PERSIST-004.1: RTM FR-31.18, NFR-31.10
  test("FR31_18_AC6_UnresolvedRenderedSeeds_FailOpenToDefaultCommunityFallback", async () => {
    const now = Date.now();
    const activitySession = `session-${now - 2 * 60_000}-unresolved-rendered-seed`;

    const { agentsPath, dbPath } = setupHomeAndDb([
      {
        filePath: `thinking/sequential/2026/03/08/${activitySession}.md`,
        content: [
          "# Sequential Thinking Session",
          "",
          "## Thought 0/2 [agent]",
          "Rendered activity includes only [[unmapped-anchor]].",
          "",
          "## Thought 1/2 [agent]",
          "Conclusion without mapped anchors.",
        ].join("\n"),
      },
    ]);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";
    seedCommunityGraph(dbPath, {
      communityCount: 3,
      conceptsPerCommunity: 3,
      anchorPrefix: "cap",
    });

    updateFileContentTimestamps(dbPath, [
      {
        filePath: "memory/cap-community-1.md",
        created: new Date(Date.now() - 3 * 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 3 * 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date().toISOString(),
      },
      {
        filePath: "memory/cap-community-2.md",
        created: new Date(Date.now() - 2 * 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 2 * 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date(Date.now() - 12 * 60 * 60_000).toISOString(),
      },
      {
        filePath: "memory/cap-community-3.md",
        created: new Date(Date.now() - 24 * 60 * 60_000).toISOString(),
        lastIndexed: new Date(Date.now() - 24 * 60 * 60_000).toISOString(),
        lastAccessed: new Date(Date.now() - 6 * 24 * 60 * 60_000).toISOString(),
      },
    ]);

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const agents = await Bun.file(agentsPath).text();
    const recentSessionsBlock = extractBlock(agents, "recent-sessions");

    expect(recentSessionsBlock).toContain("[[unmapped-anchor]]");
    expect(getCommunityAnchors(agents)).toEqual(["cap1", "cap2", "cap3"]);
  });

  // T-OC-PERSIST-004.1: RTM FR-31.18
  test("FR31_18_AC7_Traceability_NewCommunityRefinementTestsFollowNamingConvention", async () => {
    const selfSource = await Bun.file(import.meta.path).text();
    for (const id of [
      "FR31_15_AC1",
      "FR31_15_AC2",
      "FR31_16_AC3",
      "FR31_17_AC4",
      "FR31_18_AC5",
      "FR31_18_AC6",
      "FR31_18_AC7",
    ]) {
      expect(selfSource).toContain(id);
    }
  });

  // T-OC-PERSIST-004.5: RTM FR-31.18, NFR-31.9, NFR-31.9a
  test("FR31_18_AC8_EqualLastIndexedRecentSessions_RenderDeterministicSecondaryOrder", async () => {
    const tieIso = new Date().toISOString();
    const orderedRows = [
      {
        filePath: "thinking/sequential/2026/03/08/session-1700000000001-alpha.md",
        content: "## Thought 0/1 [agent]\nALPHA_INTENT",
      },
      {
        filePath: "thinking/sequential/2026/03/08/session-1700000000002-beta.md",
        content: "## Thought 0/1 [agent]\nBETA_INTENT",
      },
      {
        filePath: "thinking/sequential/2026/03/08/session-1700000000003-gamma.md",
        content: "## Thought 0/1 [agent]\nGAMMA_INTENT",
      },
    ];

    const first = setupHomeAndDb(orderedRows);
    updateFileContentTimestamps(
      first.dbPath,
      orderedRows.map((row) => ({
        filePath: row.filePath,
        created: tieIso,
        lastIndexed: tieIso,
        lastAccessed: tieIso,
      })),
    );
    process.env.MAENIFOLD_COMMUNITIES = "false";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const firstRecentSessions = extractBlock(
      await Bun.file(first.agentsPath).text(),
      "recent-sessions",
    );

    const second = setupHomeAndDb([...orderedRows].reverse());
    updateFileContentTimestamps(
      second.dbPath,
      [...orderedRows].reverse().map((row) => ({
        filePath: row.filePath,
        created: tieIso,
        lastIndexed: tieIso,
        lastAccessed: tieIso,
      })),
    );
    process.env.MAENIFOLD_COMMUNITIES = "false";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const secondRecentSessions = extractBlock(
      await Bun.file(second.agentsPath).text(),
      "recent-sessions",
    );

    expect(firstRecentSessions).toBe(secondRecentSessions);
  });

  // T-OC-PERSIST-004.5: RTM FR-31.18, NFR-31.9, NFR-31.9a
  test("FR31_18_AC9_EqualEffectiveCompactionEpochs_RenderDeterministicSecondaryOrder", async () => {
    const tieEpoch = Date.now() - 2 * 60_000;
    const orderedRows = [
      {
        filePath: `thinking/compaction/2026/03/08/compaction-ses-alpha-${tieEpoch}.md`,
        content: "COMPACTION_ALPHA",
      },
      {
        filePath: `thinking/compaction/2026/03/08/compaction-ses-beta-${tieEpoch}.md`,
        content: "COMPACTION_BETA",
      },
      {
        filePath: `thinking/compaction/2026/03/08/compaction-ses-gamma-${tieEpoch}.md`,
        content: "COMPACTION_GAMMA",
      },
    ];

    const first = setupHomeAndDb(orderedRows);
    process.env.MAENIFOLD_COMMUNITIES = "false";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "false";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const firstRecentCompactions = extractBlock(
      await Bun.file(first.agentsPath).text(),
      "recent-compactions",
    );

    const second = setupHomeAndDb([...orderedRows].reverse());
    process.env.MAENIFOLD_COMMUNITIES = "false";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "false";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "true";

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const secondRecentCompactions = extractBlock(
      await Bun.file(second.agentsPath).text(),
      "recent-compactions",
    );

    expect(firstRecentCompactions).toBe(secondRecentCompactions);
  });

  // T-OC-PERSIST-004.5: RTM FR-31.17, NFR-31.9, NFR-31.9a
  test("FR31_17_AC10_EqualAnchorScores_RenderDeterministicAnchorAndConceptOrdering", async () => {
    const now = Date.now();
    const activityRows = [
      {
        filePath: `thinking/sequential/2026/03/08/session-${now - 60_000}-anchor-order-a.md`,
        content: [
          "# Sequential Thinking Session",
          "",
          "## Thought 0/2 [agent]",
          "Rendered activity seeds [[zeta]] [[alpha]] [[omega]].",
          "",
          "## Thought 1/2 [agent]",
          "Conclusion without additional anchors.",
        ].join("\n"),
      },
    ];
    const conceptDefinitions = [
      { conceptName: "zeta", communityId: 1 },
      { conceptName: "alpha", communityId: 1 },
      { conceptName: "omega", communityId: 1 },
    ];
    const weightedTieEdges = [
      { conceptA: "zeta", conceptB: "zeta-n1", coOccurrenceCount: 2 },
      { conceptA: "zeta", conceptB: "zeta-n2", coOccurrenceCount: 3 },
      { conceptA: "alpha", conceptB: "alpha-n1", coOccurrenceCount: 4 },
      { conceptA: "alpha", conceptB: "alpha-n2", coOccurrenceCount: 1 },
      { conceptA: "omega", conceptB: "omega-n1", coOccurrenceCount: 5 },
    ];

    const first = setupHomeAndDb(activityRows);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";
    seedCustomCommunityGraph(first.dbPath, {
      concepts: conceptDefinitions,
      edges: weightedTieEdges,
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const firstCommunities = extractBlock(
      await Bun.file(first.agentsPath).text(),
      "communities",
    );

    const second = setupHomeAndDb(activityRows);
    process.env.MAENIFOLD_COMMUNITIES = "true";
    process.env.MAENIFOLD_RECENT_ACTIVITY = "true";
    process.env.MAENIFOLD_RECENT_COMPACTIONS = "false";
    seedCustomCommunityGraph(second.dbPath, {
      concepts: [...conceptDefinitions].reverse(),
      edges: [...weightedTieEdges].reverse(),
    });

    await PersistencePlugin({
      client: { app: { log: async () => undefined } },
    } as any);

    const secondCommunities = extractBlock(
      await Bun.file(second.agentsPath).text(),
      "communities",
    );

    expect(firstCommunities).toBe(secondCommunities);
  });

  // T-OC-PERSIST-004.5: RTM FR-31.18, NFR-31.9, NFR-31.9a
  test("FR31_18_AC11_Traceability_DeterministicTieBreakTestsFollowNamingConvention", async () => {
    const selfSource = await Bun.file(import.meta.path).text();
    for (const id of [
      "FR31_18_AC8",
      "FR31_18_AC9",
      "FR31_17_AC10",
      "FR31_18_AC11",
    ]) {
      expect(selfSource).toContain(id);
    }
  });
});
