import { Database } from "bun:sqlite";
import type { Plugin } from "@opencode-ai/plugin";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const _SERVICE = "opencode-plugin-persistence";
const _MAENIFOLD_ROOT =
  process.env.MAENIFOLD_ROOT ?? `${process.env.HOME}/maenifold`;
const _DB_PATH = `${_MAENIFOLD_ROOT}/memory.db`;
const _AGENTS_MD = `${process.env.HOME}/.config/opencode/AGENTS.md`;
const _GUARD_BEGIN = "<!-- BEGIN:MAENIFOLD -->";
const _GUARD_END = "<!-- END:MAENIFOLD -->";
const _SUMMARY_MAX_CHARS = 32_000;
const _PAYLOAD_MAX_CHARS = 50_000;
const _CLI_TIMEOUT_MS = 30_000;

// ---------------------------------------------------------------------------
// State — in-memory, resets on OpenCode restart
// ---------------------------------------------------------------------------

const _sequentialByProject = new Map<
  string,
  { sessionId: string; thoughtNumber: number }
>();
let _refreshTimer: ReturnType<typeof setTimeout> | null = null;
const _REFRESH_DEBOUNCE_MS = 2000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function _extractTextParts(parts: any[]): string {
  return (parts ?? [])
    .filter((p) => p?.type === "text" && typeof p.text === "string")
    .map((p) => p.text)
    .join("\n")
    .trim();
}

function _capText(
  text: string,
  maxChars: number,
  marker: string,
): { text: string; truncated: boolean } {
  if (text.length <= maxChars) return { text, truncated: false };
  return { text: `${text.slice(0, maxChars)}\n\n${marker}`, truncated: true };
}

function _containsWikiLink(text: string): boolean {
  return /\[\[[^\]]+\]\]/.test(text);
}

// ---------------------------------------------------------------------------
// WikiLink filter helpers
// ---------------------------------------------------------------------------

async function _readWikilinkFilter(): Promise<Set<string>> {
  const filterPath = `${_MAENIFOLD_ROOT}/memory/.wikilink-filter.json`;
  try {
    const text = await Bun.file(filterPath).text();
    const obj = JSON.parse(text) as Record<string, unknown>;
    return new Set(Object.keys(obj));
  } catch {
    return new Set<string>();
  }
}

function _filterWikiLinks(
  text: string,
  filterSet: Set<string>,
): { cleanedText: string; strippedTerms: string[]; survivingCount: number } {
  const regex = /\[\[([^\]]+)\]\]/g;
  const strippedTerms: string[] = [];
  let survivingCount = 0;

  const cleanedText = text.replace(regex, (_match, inner: string) => {
    const normalized = inner.trim().toLowerCase().replace(/\s+/g, "-");
    if (filterSet.has(normalized)) {
      strippedTerms.push(inner.trim());
      return inner.trim(); // strip brackets, keep plain text
    }
    survivingCount++;
    return _match; // keep as-is
  });

  return { cleanedText, strippedTerms, survivingCount };
}

function _sanitizeForStorage(text: string): string {
  return text
    .normalize("NFKC")
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g, "")
    .trim();
}

function _parseMaenifoldOutput(stdout: string): {
  success: boolean;
  sessionId?: string;
  error?: string;
} {
  const trimmed = stdout.trim();
  if (trimmed.startsWith("{")) {
    try {
      const parsed = JSON.parse(trimmed);
      if (parsed.success && parsed.data?.sessionId)
        return { success: true, sessionId: parsed.data.sessionId };
      return {
        success: false,
        error: parsed.error?.message ?? "Unknown maenifold error",
      };
    } catch {
      /* fall through */
    }
  }
  if (trimmed.startsWith("ERROR:")) return { success: false, error: trimmed };
  const match = trimmed.match(/session-[\w-]+/);
  if (match) return { success: true, sessionId: match[0] };
  return {
    success: false,
    error: `Unexpected output: ${trimmed.slice(0, 200)}`,
  };
}

// ---------------------------------------------------------------------------
// Content extraction — first/last thought or step
// ---------------------------------------------------------------------------

function _extractFirstLast(
  content: string,
  kind: string,
): { first: string | null; last: string | null; count: number } {
  if (!content) return { first: null, last: null, count: 0 };

  if (kind === "workflow") {
    // Split on ## Step N headers; produces pairs: definition, response
    const blocks = content.split(/^## Step \d+/m).slice(1);
    // Definitions are blocks starting with /N\n (no "Response")
    // Responses are blocks starting with /N Response\n
    const defs: string[] = [];
    const responses: string[] = [];
    for (const b of blocks) {
      if (/^\/\d+\s+Response/i.test(b)) {
        responses.push(b);
      } else {
        defs.push(b);
      }
    }
    const stepCount = Math.max(defs.length, responses.length);
    const first = _workflowDefLabel(defs[0] ?? "");
    const last =
      responses.length > 0
        ? _firstTextLine(responses[responses.length - 1] ?? "")
        : null;
    return { first, last, count: stepCount };
  }

  // Sequential: split on ## Thought N headers
  const thoughts = content.split(/^## Thought \d+/m).slice(1);
  const count = thoughts.length;
  const first = _firstTextLine(thoughts[0] ?? "");
  const last = count > 1 ? _firstTextLine(thoughts[count - 1] ?? "") : null;
  return { first, last, count };
}

/** Extract the "name" field from a workflow step definition block. */
function _workflowDefLabel(block: string): string | null {
  const nameMatch = block.match(/"name"\s*:\s*"([^"]+)"/);
  if (nameMatch) {
    return nameMatch[1]
      .replace(/[\p{Emoji_Presentation}\p{Emoji}\uFE0F]/gu, "")
      .trim();
  }
  return _firstTextLine(block);
}

function _firstTextLine(block: string): string | null {
  const lines = block.split("\n");
  for (const line of lines) {
    const trimmed = line.trim();
    // Skip empty, timestamp, header-remnant, JSON, and bracket lines
    if (
      !trimmed ||
      /^\/\d+/.test(trimmed) ||
      trimmed.startsWith("*20") ||
      trimmed.startsWith("{") ||
      trimmed.startsWith("}") ||
      /^\[.*\]$/.test(trimmed) ||
      /^#+\s/.test(trimmed) ||
      /^Response$/.test(trimmed)
    )
      continue;
    return trimmed;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Graph context — direct SQLite reads
// ---------------------------------------------------------------------------

function _openDb(): Database | null {
  try {
    const db = new Database(_DB_PATH);
    db.exec("PRAGMA read_uncommitted=ON;");
    return db;
  } catch (err) {
    console.error(`[${_SERVICE}] _openDb FAILED: ${_DB_PATH} — ${err}`);
    return null;
  }
}

function _buildGraphContext(): string | null {
  const db = _openDb();
  if (!db) {
    console.error(`[${_SERVICE}] _buildGraphContext: _openDb returned null`);
    return null;
  }

  const includeCommunities = process.env.MAENIFOLD_COMMUNITIES !== "false";
  const includeRecentActivity =
    process.env.MAENIFOLD_RECENT_ACTIVITY !== "false";

  if (!includeCommunities && !includeRecentActivity) return null;

  try {
    // Community concept index — decay-weighted (ACT-R power-law)
    // Each concept's decay weight = MAX across all files mentioning it.
    // Power-law formula: weight = 1.0 * (age_days)^(-0.5) after grace period.
    // Grace periods: thinking/sequential/ = 7d, thinking/workflow/ = 14d, other = 28d.
    let priming = "";
    if (includeCommunities) {
      const communities = db
        .query<{ community_id: number; concepts: string }, []>(
          `
          WITH concept_decay AS (
            -- ACT-R power-law decay: weight = 1.0 / sqrt(effective_age) after grace period
            -- Grace: sequential=7d, workflow=14d, other=28d. Within grace → weight=1.0.
            SELECT cm.concept_name,
              MAX(
                CASE
                  WHEN julianday('now') - julianday(fc.last_indexed) <=
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  THEN 1.0
                  ELSE MIN(1.0, 1.0 / sqrt(MAX(1,
                    julianday('now') - julianday(fc.last_indexed) -
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  )))
                END
              ) as max_decay
            FROM concept_mentions cm
            JOIN file_content fc ON cm.source_file = fc.file_path
            WHERE fc.last_indexed IS NOT NULL
            GROUP BY cm.concept_name
          ),
          community_sizes AS (
            SELECT community_id, COUNT(*) as size
            FROM concept_communities GROUP BY community_id HAVING size >= 3
          ),
          concept_degree AS (
            SELECT cc.concept_name, cc.community_id,
                   SUM(cg.co_occurrence_count) as raw_weight,
                   COALESCE(cd.max_decay, 0.1) as max_decay
            FROM concept_communities cc
            JOIN community_sizes cs ON cc.community_id = cs.community_id
            LEFT JOIN concept_graph cg
              ON cc.concept_name = cg.concept_a OR cc.concept_name = cg.concept_b
            LEFT JOIN concept_decay cd ON cc.concept_name = cd.concept_name
            WHERE cc.concept_name NOT GLOB '*[.]*'
              AND length(cc.concept_name) >= 3
              AND cc.concept_name NOT GLOB '[0-9]*'
            GROUP BY cc.concept_name, cc.community_id
          ),
          ranked AS (
            SELECT *, raw_weight * max_decay as weighted_score,
              ROW_NUMBER() OVER (
                PARTITION BY community_id ORDER BY raw_weight * max_decay DESC
              ) as rn
            FROM concept_degree WHERE raw_weight > 0
          )
          SELECT community_id, group_concat(concept_name, ' ') as concepts
          FROM ranked WHERE rn <= 20
          GROUP BY community_id
          ORDER BY MAX(weighted_score) DESC
          `,
        )
        .all();

      for (const row of communities) {
        if (!row.concepts) continue;
        const cs = row.concepts.split(" ");
        priming +=
          cs.map((c, i) => (i === 0 ? `**[[${c}]]**` : `[[${c}]]`)).join(" ") +
          "\n";
      }
    }

    // Recent thinking threads (sequential + workflow) with content
    const threadLines: string[] = [];
    if (includeRecentActivity) {
      const threads = db
        .query<
          {
            session_id: string;
            status: string | null;
            content: string | null;
            kind: string;
          },
          []
        >(
          `
          SELECT
            CASE
              WHEN file_path LIKE '%/session-%'
              THEN substr(file_path, instr(file_path, 'session-'))
              WHEN file_path LIKE '%/workflow-%'
              THEN substr(file_path, instr(file_path, 'workflow-'))
              ELSE file_path
            END as session_id,
            status,
            content,
            CASE
              WHEN file_path LIKE '%/workflow-%' THEN 'workflow'
              ELSE 'sequential'
            END as kind
          FROM file_content
          WHERE (file_path LIKE '%thinking/sequential/%session-%'
                 OR file_path LIKE '%thinking/workflow/%workflow-%')
            AND last_indexed > datetime('now', '-3 days')
          ORDER BY last_indexed DESC LIMIT 10
          `,
        )
        .all();

      for (const r of threads) {
        const { first, last, count } = _extractFirstLast(
          r.content ?? "",
          r.kind,
        );
        const header = `${r.session_id} (${r.status ?? "unknown"}, ${count}t)`;
        const parts = [header];
        if (first) parts.push(`  Q: ${first}`);
        if (last && last !== first) parts.push(`  A: ${last}`);
        threadLines.push(parts.join("\n"));
      }
    }

    const lines: string[] = [
      "# Bootstrap Protocol",
      "",
      "When starting a session and after compaction you must (re-)load your `maenifold` skill. This skill is core to your identity as an agent and you cannot function without it. If you find yourself without the skill loaded you should stop and load it before continuing",
      "Prefer using the `maenifold` CLI over the MCP server",
      "",
      "# Graph of thought",
      "",
      "Your internal memory is a cache — it resets between sessions and gets compacted mid-session. This reset is not a limitation—it forces reliance on maenifold's knowledge graph and the memory:// corpus as living infrastructure. Use all `maenifold` skills and tools to retrieve and persist knowledge as needed. Never rely on internal memory—use `buildcontext`, `findsimilarconcepts` and `searchmemories` to ground answers in the knowledge graph. When insufficient information exists for a confident recommendation, clearly state what additional data or input would help, then use external knowledge sources to research and write lineage-backed memory:// notes to inform the answer.",
      "",
      "When reasoning or responding, please tag important concepts, entities and terms as [[wikilinks]], adhering to the wiki-style tagging guidelines.",
      "maenifold will automatically extract these [[WikiLinks]] and use them to build a knowledge graph that connects concepts across sessions. This enables better context recovery and more intelligent assistance in future interactions.",
      "WikiLinks become graph nodes. Bad tagging = graph corruption = broken context recovery.",
      "",
      "Use maenifold's sequential thinking tool to think through problems and persist your resoning to it's graph for future reference. If you do something without explaining why in maenifold your future sell will not know why you did what you did.",
      "",
      "Search the graph first for existing notes to update before creating new ones.Always update existing memory:// notes instead of creating duplicates. Always search for the correct folder to place new notes to ensure memory follows the ontology and is easily discoverable later.",
      "",
      "Context will be automatically compacted as it approaches its limit. Do not stop tasks early due to token budget concerns. Save progress to memory:// when approaching the context limit and rehydrate from that location post compaction.",
    ];

    if (priming) {
      lines.push(
        "",
        "## Context Injection",
        "",
        "The graph is your true context window. It is a knowledge graph built from every `[[WikiLink]]` you and other agents have written across all sessions, domains, and tools. It encodes not just facts but reasoning — chains of thought, decisions, assumptions, and their outcomes.",
        "",
        "The concepts below are Louvain communities: clusters of co-occurring WikiLinks that represent reasoning domains. Bold = the domain anchor (highest connectivity). Each `[[WikiLink]]` is a graph node you can traverse with `build_context` to pull in related concepts, or `search_memories` to find the full documents behind them. Start from what you see here; go deeper only when you need the full prose.",
        "",
        priming.trimEnd(),
      );
    }

    if (threadLines.length > 0) {
      lines.push(
        "",
        "## Recent Activity",
        "",
        "Recent sequential thinking and workflow sessions. Q is the first thought or step (the intent). A is the last (the conclusion). Use `read_memory` with a session ID to retrieve the full session.",
        "",
        ...threadLines,
      );
    }

    return lines.join("\n");
  } finally {
    db.close();
  }
}

// ---------------------------------------------------------------------------
// AGENTS.md guarded block
// ---------------------------------------------------------------------------

async function _refreshAgentsMd(): Promise<boolean> {
  const context = _buildGraphContext();
  if (!context) return false;

  const block = `${_GUARD_BEGIN}\n${context}\n${_GUARD_END}`;

  let existing = "";
  try {
    existing = await Bun.file(_AGENTS_MD).text();
  } catch {
    // file doesn't exist yet
  }

  const beginIdx = existing.indexOf(_GUARD_BEGIN);
  const endIdx = existing.indexOf(_GUARD_END);

  let updated: string;
  if (beginIdx !== -1 && endIdx !== -1) {
    // Replace existing guarded block
    updated =
      existing.slice(0, beginIdx) +
      block +
      existing.slice(endIdx + _GUARD_END.length);
  } else {
    // Append guarded block
    updated = existing ? `${existing.trimEnd()}\n\n${block}\n` : `${block}\n`;
  }

  await Bun.write(_AGENTS_MD, updated);
  return true;
}

// ---------------------------------------------------------------------------
// Compaction persistence
// ---------------------------------------------------------------------------

async function _persistCompaction(
  client: any,
  sessionID: string,
  projectKey: string,
): Promise<void> {
  const result = await client.session.messages({ path: { id: sessionID } });
  const messages: any[] = Array.isArray(result) ? result : (result?.data ?? []);

  const summaryMsg = [...messages]
    .reverse()
    .find((m) => m?.info?.role === "assistant" && m?.info?.summary === true);

  const raw = summaryMsg ? _extractTextParts(summaryMsg.parts) : "";
  if (!raw) {
    await client.app.log({
      body: {
        service: _SERVICE,
        level: "warn",
        message: "No summary message found after compaction",
        extra: { sessionID },
      },
    });
    return;
  }

  const sanitized = _sanitizeForStorage(raw);
  const capped = _capText(
    sanitized,
    _SUMMARY_MAX_CHARS,
    `[TRUNCATED at ${_SUMMARY_MAX_CHARS} chars]`,
  );

  const prior = _sequentialByProject.get(projectKey);
  const thoughtNumber = prior ? prior.thoughtNumber + 1 : 0;

  // Filter out WikiLinks that are in the wikilink-filter (they'd be stripped
  // server-side causing "Must include [[WikiLinks]]" rejection).
  const filterSet = await _readWikilinkFilter();
  const { cleanedText, strippedTerms, survivingCount } = _filterWikiLinks(
    capped.text,
    filterSet,
  );

  // Notify the agent about stripped terms so they're not invisible.
  if (strippedTerms.length > 0) {
    const strippedList = strippedTerms.map((t) => `[[${t}]]`).join(", ");
    await client.tui.showToast({
      body: {
        title: "WikiLink filter",
        message: `WikiLink filter stripped: ${strippedList}`,
        variant: "info",
        duration: 6000,
      },
    });
  }

  // Add fallback prefix AFTER filtering (prefix terms are not in the filter).
  // If survivingCount is still 0 after filtering, the prefix rescues the write.
  const prefix = survivingCount > 0 ? "" : "[[opencode]] [[compaction]]\n\n";

  const response = [
    prefix,
    "## OpenCode compaction summary (untrusted)",
    "",
    "```",
    cleanedText,
    "```",
  ].join("\n");

  // If the filtered text has no WikiLinks AND the prefix is also absent
  // (shouldn't happen since opencode/compaction are not filtered), skip.
  if (survivingCount === 0 && prefix === "") {
    await client.app.log({
      body: {
        service: _SERVICE,
        level: "warn",
        message:
          "Compaction summary has no surviving WikiLinks after filtering; skipping CLI call",
        extra: { sessionID, strippedTerms },
      },
    });
    return;
  }

  const payload: Record<string, unknown> = {
    response,
    nextThoughtNeeded: true,
    thoughtNumber,
    totalThoughts: 0,
  };
  if (prior) payload.sessionId = prior.sessionId;

  const payloadJson = JSON.stringify(payload);
  if (payloadJson.length > _PAYLOAD_MAX_CHARS) {
    await client.app.log({
      body: {
        service: _SERVICE,
        level: "warn",
        message: `Payload too large (${payloadJson.length} chars); skipping`,
        extra: { sessionID, thoughtNumber },
      },
    });
    return;
  }

  const proc = Bun.spawn(
    [
      "maenifold",
      "--tool",
      "SequentialThinking",
      "--payload",
      payloadJson,
      "--json",
    ],
    { stdout: "pipe", stderr: "pipe", timeout: _CLI_TIMEOUT_MS },
  );

  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);

  if (proc.signalCode) {
    throw new Error(`maenifold killed by ${proc.signalCode} (timeout)`);
  }
  if (exitCode !== 0) {
    throw new Error(`maenifold exit ${exitCode}: ${stderr || stdout}`);
  }

  const parsed = _parseMaenifoldOutput(stdout);
  if (!parsed.success) throw new Error(parsed.error);

  const updatedSessionId = parsed.sessionId ?? prior?.sessionId;
  if (!updatedSessionId) throw new Error("No sessionId in maenifold output");

  _sequentialByProject.set(projectKey, {
    sessionId: updatedSessionId,
    thoughtNumber,
  });

  await client.app.log({
    body: {
      service: _SERVICE,
      level: "info",
      message: `Compaction persisted (thought ${thoughtNumber})`,
      extra: { sessionID, maenifoldSessionId: updatedSessionId },
    },
  });
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

export const PersistencePlugin: Plugin = async ({
  project,
  client,
  directory,
  worktree,
}) => {
  const projectKey = project?.id ?? worktree ?? directory ?? "unknown";

  // Seed AGENTS.md with current graph context on startup
  console.error(`[${_SERVICE}] init: MAENIFOLD_ROOT=${_MAENIFOLD_ROOT}, DB_PATH=${_DB_PATH}`);
  try {
    const wrote = await _refreshAgentsMd();
    console.error(`[${_SERVICE}] _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED (returned false)"}`);
    await client.app.log({
      body: {
        service: _SERVICE,
        level: "info",
        message: wrote
          ? "AGENTS.md graph context seeded"
          : "No graph data available for AGENTS.md",
      },
    });
  } catch (err) {
    console.error(`[${_SERVICE}] _refreshAgentsMd FAILED: ${err}`);
    await client.app.log({
      body: {
        service: _SERVICE,
        level: "warn",
        message: `AGENTS.md seed failed: ${err instanceof Error ? err.message : err}`,
      },
    });
  }

  return {
    event: async ({ event }) => {
      // Event logging disabled — enable for debugging

      // --- session.created: refresh immediately ---
      if (event.type === "session.created") {
        try {
          const wrote = await _refreshAgentsMd();
          console.error(`[${_SERVICE}] session.created _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`);
        } catch (err) {
          console.error(`[${_SERVICE}] session.created _refreshAgentsMd FAILED: ${err}`);
        }
        return;
      }

      // --- session.updated: debounced refresh ---
      if (event.type === "session.updated") {
        if (_refreshTimer) clearTimeout(_refreshTimer);
        _refreshTimer = setTimeout(async () => {
          try {
            const wrote = await _refreshAgentsMd();
            console.error(`[${_SERVICE}] session.updated _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`);
          } catch (err) {
            console.error(`[${_SERVICE}] session.updated _refreshAgentsMd FAILED: ${err}`);
          }
        }, _REFRESH_DEBOUNCE_MS);
        return;
      }

      // --- session.compacted: persist summary, then refresh ---
      if (event.type === "session.compacted") {
        const sessionID = (event.properties as any)?.sessionID;
        console.error(`[${_SERVICE}] session.compacted: sessionID=${sessionID}`);
        if (typeof sessionID !== "string") return;

        // 1. Persist compaction summary
        try {
          await _persistCompaction(client, sessionID, projectKey);
          console.error(`[${_SERVICE}] _persistCompaction: OK`);
        } catch (err) {
          console.error(`[${_SERVICE}] _persistCompaction FAILED: ${err}`);
          try {
            await client.app.log({
              body: {
                service: _SERVICE,
                level: "error",
                message: `Compaction persistence failed: ${err instanceof Error ? err.message : err}`,
                extra: { sessionID },
              },
            });
          } catch {
            /* ignore */
          }
          await client.tui.showToast({
            body: {
              title: "Persistence",
              message: `Failed: ${err instanceof Error ? err.message : err}`,
              variant: "error",
              duration: 8000,
            },
          });
        }

        // 2. Refresh AGENTS.md with fresh graph data (includes persisted summary)
        try {
          const wrote = await _refreshAgentsMd();
          console.error(`[${_SERVICE}] post-compaction _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`);
        } catch (err) {
          console.error(`[${_SERVICE}] post-compaction _refreshAgentsMd FAILED: ${err}`);
          await client.app.log({
            body: {
              service: _SERVICE,
              level: "warn",
              message: `AGENTS.md refresh failed after compaction: ${err instanceof Error ? err.message : err}`,
            },
          });
        }

        await client.tui.showToast({
          body: {
            title: "Persistence",
            message: "Compaction saved, graph context refreshed",
            variant: "success",
            duration: 4000,
          },
        });
        return;
      }
    },
  };
};
