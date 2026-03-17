import { Database } from "bun:sqlite";
import {
  accessSync,
  constants as fsConstants,
  existsSync,
  statSync,
} from "node:fs";
import type { Plugin } from "@opencode-ai/plugin";
import { getMaenifoldPluginEnv, getMaenifoldPluginFlag } from "./lib/config";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const _SERVICE = "opencode-plugin-persistence";
const _GUARD_BEGIN = "<!-- BEGIN:MAENIFOLD -->";
const _GUARD_END = "<!-- END:MAENIFOLD -->";
const _SUMMARY_MAX_CHARS = 32_000;
const _PAYLOAD_MAX_CHARS = 50_000;
const _CLI_TIMEOUT_MS = 30_000;
const _DEBUG = getMaenifoldPluginFlag("MAENIFOLD_DEBUG") === true;

// ---------------------------------------------------------------------------
// State — in-memory, resets on OpenCode restart
// ---------------------------------------------------------------------------

let _refreshTimer: ReturnType<typeof setTimeout> | null = null;
const _REFRESH_DEBOUNCE_MS = 2000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function _getDefaultRoot(): string | null {
  const home = process.env.HOME;
  if (!home || home.trim().length === 0) return null;
  return `${home.trim()}/maenifold`;
}

function _getMaenifoldRoot(): string | null {
  const configured = getMaenifoldPluginEnv("MAENIFOLD_ROOT")?.trim();
  if (configured) return configured;
  return _getDefaultRoot();
}

function _getDbPath(): string | null {
  const configured = getMaenifoldPluginEnv("MAENIFOLD_DATABASE_PATH")?.trim();
  if (configured) return configured;
  const root = _getMaenifoldRoot();
  return root ? `${root}/memory.db` : null;
}

function _isRecentActivityEnabled(): boolean {
  return getMaenifoldPluginFlag("MAENIFOLD_RECENT_ACTIVITY") === true;
}

function _isRecentCompactionsEnabled(): boolean {
  return getMaenifoldPluginFlag("MAENIFOLD_RECENT_COMPACTIONS") !== false;
}

function _getAgentsMdPath(): string | null {
  const home = process.env.HOME?.trim();
  if (!home) return null;
  return `${home}/.config/opencode/AGENTS.md`;
}

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

// ---------------------------------------------------------------------------
// WikiLink filter helpers
// ---------------------------------------------------------------------------

async function _readWikilinkFilter(): Promise<Set<string>> {
  const maenifoldRoot = _getMaenifoldRoot();
  if (!maenifoldRoot) return new Set<string>();
  const filterPath = `${maenifoldRoot}/memory/.wikilink-filter.json`;
  try {
    const text = await Bun.file(filterPath).text();
    const obj = JSON.parse(text) as Record<string, unknown>;
    return new Set(Object.keys(obj));
  } catch (err: unknown) {
    if (
      err instanceof Error &&
      "code" in err &&
      (err as any).code === "ENOENT"
    ) {
      return new Set<string>();
    }
    throw err;
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

function _parseMaenifoldOutput(stdout: string): { uri: string } {
  const parsed = JSON.parse(stdout.trim());
  if (!parsed.success) {
    throw new Error(parsed.error?.message ?? "Unknown maenifold error");
  }
  if (!parsed.data?.uri) {
    throw new Error("maenifold returned success but no uri");
  }
  return { uri: parsed.data.uri };
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

function _createReadonlyDb(dbPath: string): Database {
  const db = new Database(dbPath);
  db.exec("PRAGMA query_only=ON;");
  db.exec("PRAGMA busy_timeout=5000;");
  return db;
}

type _DbOpener = (dbPath: string) => Database;

function _openDb(openDb: _DbOpener = _createReadonlyDb): Database | null {
  const dbPath = _getDbPath();
  if (!dbPath) {
    console.error(
      `[${_SERVICE}] _openDb FAILED: DB path not set (set MAENIFOLD_DATABASE_PATH or MAENIFOLD_ROOT)`,
    );
    return null;
  }
  if (!existsSync(dbPath)) {
    console.error(`[${_SERVICE}] _openDb FAILED: ${dbPath} does not exist`);
    return null;
  }
  try {
    const stat = statSync(dbPath);
    if (!stat.isFile()) {
      console.error(`[${_SERVICE}] _openDb FAILED: ${dbPath} is not a file`);
      return null;
    }
  } catch (err) {
    console.error(`[${_SERVICE}] _openDb FAILED: ${dbPath} — ${err}`);
    return null;
  }
  try {
    accessSync(dbPath, fsConstants.R_OK);
  } catch (err) {
    console.error(
      `[${_SERVICE}] _openDb FAILED: ${dbPath} is not readable — ${err}`,
    );
    return null;
  }
  try {
    const db = openDb(dbPath);
    db.exec("PRAGMA read_uncommitted=ON;");
    return db;
  } catch (err) {
    console.error(`[${_SERVICE}] _openDb FAILED: ${dbPath} — ${err}`);
    return null;
  }
}

type _CompactionEntry = {
  sessionId: string;
  sessionEpoch: number | null;
  compactionEpoch: number;
  filePath: string;
  content: string | null;
};

type _CompactionRow = {
  file_path: string;
  content: string | null;
  created: string | null;
  last_indexed: string | null;
};

type _CommunityRow = {
  community_id: number;
  concept_name: string;
  rn: number;
  anchor: string;
  anchor_score: number;
};

type _CommunityRecencyRow = {
  community_id: number;
  community_updated_at: string | null;
};

type _CommunityEntry = {
  communityId: number;
  anchor: string;
  concepts: string[];
  anchorScore: number;
};

function _parseDateEpoch(value: string | null | undefined): number | null {
  if (!value) return null;
  const epoch = Date.parse(value);
  return Number.isFinite(epoch) ? epoch : null;
}

function _normalizeMemoryUri(filePath: string): string {
  const withoutExtension = filePath.replace(/\.md$/, "");
  return withoutExtension.startsWith("memory://")
    ? withoutExtension
    : `memory://${withoutExtension}`;
}

function _encodeSessionTokenHex(rawSessionId: string): string {
  return Buffer.from(rawSessionId, "utf8").toString("hex");
}

function _decodeSessionTokenHex(token: string): string | null {
  if (!/^[0-9a-f]+$/i.test(token) || token.length % 2 !== 0) return null;

  try {
    const decoded = Buffer.from(token, "hex").toString("utf8");
    if (!decoded || decoded.trim().length === 0) return null;
    if (_encodeSessionTokenHex(decoded) !== token.toLowerCase()) return null;
    return decoded;
  } catch {
    return null;
  }
}

function _parseCompactionEntry(
  filePath: string,
  content: string | null,
  created: string | null,
  lastIndexed: string | null,
): _CompactionEntry | null {
  const filename = filePath.split("/").pop() ?? "";
  const basename = filename.endsWith(".md")
    ? filename.slice(0, -".md".length)
    : filename;
  const fileEpoch = _parseDateEpoch(created) ?? _parseDateEpoch(lastIndexed);

  const sidHexMatch = basename.match(
    /^compaction-sidhex-([0-9a-f]+)-ts-(\d+)$/i,
  );
  if (sidHexMatch) {
    const sessionId = _decodeSessionTokenHex(sidHexMatch[1]);
    if (!sessionId) return null;

    const compactionEpoch = Number(sidHexMatch[2]);
    if (!Number.isFinite(compactionEpoch)) return null;

    const sessionEpoch = _extractSessionEpoch(sessionId);
    return { sessionId, sessionEpoch, compactionEpoch, filePath, content };
  }

  // Preferred canonical format (collision-safe token in title):
  // compaction-<sessionSlug>--sid-<base64url(rawSessionId)>-<epochMillis>
  const tokenizedMatch = basename.match(
    /^compaction-(.+)--sid-([A-Za-z0-9_-]+)-(\d+)$/,
  );
  if (tokenizedMatch) {
    const sessionId = _decodeSessionToken(tokenizedMatch[2]);
    if (!sessionId) return null;

    const compactionEpoch = Number(tokenizedMatch[3]);
    if (!Number.isFinite(compactionEpoch)) return null;

    const sessionEpoch = _extractSessionEpoch(sessionId);
    return { sessionId, sessionEpoch, compactionEpoch, filePath, content };
  }

  // Legacy canonical format: compaction-<sessionIdOrSlug>-<epochMillis>
  // Keep fail-open behavior for malformed historical rows.
  const legacyMatch = basename.match(/^compaction-(.+)-(\d+)$/);
  if (!legacyMatch) return null;

  const sessionIdFromTitle = (legacyMatch[1] ?? "").trim();
  const compactionEpoch = Number(legacyMatch[2]);
  if (!Number.isFinite(compactionEpoch)) return null;

  // Legacy compatibility: if title identity is lossy, preserve row isolation
  // by preferring persisted raw session metadata for grouping only.
  const rawSessionId = _extractRawSessionIdFromContent(content);
  const sessionId = rawSessionId ?? sessionIdFromTitle;

  // Recency/session epoch MUST come from trusted title-derived identity only.
  const sessionEpoch = _extractSessionEpoch(sessionIdFromTitle);

  return { sessionId, sessionEpoch, compactionEpoch, filePath, content };
}

// Current real-world WriteMemory slug behavior strips title hyphen separators, so
// machine-authored compaction files can be stored as `compactionses-...177299...`.
// In that fallback mode, use trusted DB timestamps for recency and content metadata
// only for grouping.
function _parseFallbackCompactionEntry(
  filePath: string,
  content: string | null,
  created: string | null,
  lastIndexed: string | null,
): _CompactionEntry | null {
  const filename = filePath.split("/").pop() ?? "";
  const basename = filename.endsWith(".md")
    ? filename.slice(0, -".md".length)
    : filename;

  if (!basename.startsWith("compaction")) return null;

  const compactionEpoch =
    _parseDateEpoch(created) ?? _parseDateEpoch(lastIndexed);
  if (!compactionEpoch) return null;

  const sessionId = _extractRawSessionIdFromContent(content) ?? basename;
  return {
    sessionId,
    sessionEpoch: null,
    compactionEpoch,
    filePath,
    content,
  };
}

function _slugSessionId(rawSessionId: string): string {
  const normalized = rawSessionId
    .normalize("NFKC")
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+|-+$/g, "");

  return normalized.length > 0 ? normalized : "session";
}

function _isFilenameSafeSessionToken(value: string): boolean {
  return /^[A-Za-z0-9._-]+$/.test(value);
}

function _sessionTokenForTitle(rawSessionId: string): string {
  const normalized = rawSessionId.normalize("NFKC").trim();
  if (normalized.length === 0) return "session";

  // Prefer non-lossy token when filename-safe (backward compatible).
  if (_isFilenameSafeSessionToken(normalized)) {
    return normalized;
  }

  // Fallback for opaque IDs containing unsafe filename characters.
  return `sid-${_encodeSessionToken(normalized)}`;
}

function _encodeSessionToken(rawSessionId: string): string {
  return Buffer.from(rawSessionId, "utf8").toString("base64url");
}

function _decodeSessionToken(token: string): string | null {
  try {
    const decoded = Buffer.from(token, "base64url").toString("utf8");
    if (!decoded || decoded.trim().length === 0) return null;

    // Canonicalization guard: token must round-trip exactly.
    if (_encodeSessionToken(decoded) !== token) return null;
    return decoded;
  } catch {
    return null;
  }
}

function _extractRawSessionIdFromContent(
  content: string | null,
): string | null {
  if (!content) return null;

  // Current metadata line format:
  // Raw Session ID: <id>
  const lineMatch = content.match(/^Raw Session ID:\s*(.+)$/m);
  if (!lineMatch?.[1]) return null;
  const raw = lineMatch[1].trim();
  return raw.length > 0 ? raw : null;
}

function _extractSessionEpoch(sessionIdentityFromTitle: string): number | null {
  // Canonical legacy session shape: session-<epoch>-...
  const canonical = sessionIdentityFromTitle.match(/^session-(\d{10,})-.+$/);
  if (canonical?.[1]) {
    const epoch = Number(canonical[1]);
    if (Number.isFinite(epoch)) return epoch;
  }

  // Separator-collapsed legacy variants: session<epoch>...
  const collapsed = sessionIdentityFromTitle.match(
    /^session(\d{10,})(?:-.+|.+)?$/,
  );
  if (collapsed?.[1]) {
    const epoch = Number(collapsed[1]);
    if (Number.isFinite(epoch)) return epoch;
  }

  return null;
}

function _escapeGuardDelimitersForAgents(content: string): string {
  return content
    .replace(/<!-- BEGIN:MAENIFOLD -->/g, "&lt;!-- BEGIN:MAENIFOLD --&gt;")
    .replace(/<!-- END:MAENIFOLD -->/g, "&lt;!-- END:MAENIFOLD --&gt;");
}

function _extractWikiLinks(text: string): Set<string> {
  const links = new Set<string>();
  if (!text) return links;

  const regex = /\[\[([^\]]+)\]\]/g;
  for (const match of text.matchAll(regex)) {
    const raw = (match[1] ?? "").trim();
    if (!raw) continue;
    const normalized = raw.toLowerCase().replace(/\s+/g, "-");
    if (!normalized) continue;
    links.add(normalized);
  }

  return links;
}

function _buildCommunityEntries(rows: _CommunityRow[]): {
  communities: _CommunityEntry[];
  communityById: Map<number, _CommunityEntry>;
  anchorByCommunityId: Map<number, string>;
} {
  const byCommunity = new Map<number, _CommunityEntry>();
  const anchorByCommunityId = new Map<number, string>();

  for (const row of rows) {
    let entry = byCommunity.get(row.community_id);
    if (!entry) {
      entry = {
        communityId: row.community_id,
        anchor: row.anchor,
        concepts: [],
        anchorScore: row.anchor_score,
      };
      byCommunity.set(row.community_id, entry);
      anchorByCommunityId.set(row.community_id, row.anchor);
    }

    if (entry.concepts.length < 10) {
      entry.concepts.push(row.concept_name);
    }
  }

  const communities = [...byCommunity.values()]
    .sort(
      (a, b) =>
        b.anchorScore - a.anchorScore ||
        a.communityId - b.communityId ||
        a.anchor.localeCompare(b.anchor),
    )
    .slice(0, Number.MAX_SAFE_INTEGER);

  return { communities, communityById: byCommunity, anchorByCommunityId };
}

function _collectRenderedSourceSeeds(renderedSources: string[]): Set<string> {
  const seeds = new Set<string>();

  for (const renderedSource of renderedSources) {
    for (const link of _extractWikiLinks(renderedSource)) {
      seeds.add(link);
    }
  }

  return seeds;
}

function _effectiveCompactionEpoch(entry: _CompactionEntry): number {
  return entry.sessionEpoch ?? entry.compactionEpoch;
}

function _compareCompactionEntries(
  a: _CompactionEntry,
  b: _CompactionEntry,
): number {
  return (
    _effectiveCompactionEpoch(b) - _effectiveCompactionEpoch(a) ||
    a.sessionId.localeCompare(b.sessionId) ||
    b.compactionEpoch - a.compactionEpoch ||
    a.filePath.localeCompare(b.filePath)
  );
}

function _preferLatestCompactionCandidate(
  candidate: _CompactionEntry,
  existing: _CompactionEntry,
): boolean {
  return (
    candidate.compactionEpoch > existing.compactionEpoch ||
    (candidate.compactionEpoch === existing.compactionEpoch &&
      candidate.filePath.localeCompare(existing.filePath) < 0)
  );
}

function _buildGraphContext(): string | null {
  const includeCommunities =
    getMaenifoldPluginFlag("MAENIFOLD_COMMUNITIES") !== false;
  const includeRecentActivity = _isRecentActivityEnabled();
  const includeRecentCompactions = _isRecentCompactionsEnabled();

  if (!includeCommunities && !includeRecentCompactions) return null;

  const db = _openDb();
  if (!db) {
    console.error(`[${_SERVICE}] _buildGraphContext: _openDb returned null`);
    return null;
  }

  try {
    // Community concept index — decay-weighted (ACT-R power-law)
    // Each concept's decay weight = MAX across all files mentioning it.
    // Power-law formula: weight = 1.0 * (age_days)^(-0.5) after grace period.
    // Reference date: last_accessed (access-boosted) ?? created ?? last_indexed
    // Grace periods: thinking/sequential|compaction/ = 7d, thinking/workflow/ = 14d, other = 28d.
    let priming = "";

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
            AND file_path NOT LIKE '%thinking/compaction/%'
            AND last_indexed > datetime('now', '-3 days')
          ORDER BY last_indexed DESC, file_path ASC LIMIT 10
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

    // Recent compactions (latest artifact per recent session cohort)
    const recentCompactionLines: string[] = [];
    if (includeRecentCompactions) {
      const nowMs = Date.now();
      const windowStartMs = nowMs - 30 * 60_000;

      const rows = db
        .query<_CompactionRow, []>(
          `
          SELECT file_path, content, created, last_indexed
          FROM file_content
          WHERE file_path LIKE '%thinking/compaction/%'
          `,
        )
        .all();

      const latestBySession = new Map<string, _CompactionEntry>();
      for (const row of rows) {
        try {
          const parsed =
            _parseCompactionEntry(
              row.file_path,
              row.content,
              row.created,
              row.last_indexed,
            ) ??
            _parseFallbackCompactionEntry(
              row.file_path,
              row.content,
              row.created,
              row.last_indexed,
            );
          if (!parsed) continue;
          const effectiveEpoch = parsed.sessionEpoch ?? parsed.compactionEpoch;
          if (effectiveEpoch < windowStartMs || effectiveEpoch > nowMs) {
            continue;
          }

          const existing = latestBySession.get(parsed.sessionId);
          if (!existing || _preferLatestCompactionCandidate(parsed, existing)) {
            latestBySession.set(parsed.sessionId, parsed);
          }
        } catch {
          // Fail-open: skip malformed rows without throwing.
        }
      }

      const selected = [...latestBySession.values()]
        .sort(_compareCompactionEntries)
        .slice(0, 10);

      for (const entry of selected) {
        const memoryUri = _normalizeMemoryUri(entry.filePath);
        recentCompactionLines.push(memoryUri);
        if (entry.content) {
          recentCompactionLines.push(
            _escapeGuardDelimitersForAgents(entry.content),
          );
        }
      }
    }

    if (includeCommunities) {
      const communityRows = db
        .query<_CommunityRow, []>(
          `
          WITH concept_decay AS (
            -- ACT-R power-law decay: weight = 1.0 / sqrt(effective_age) after grace period
            -- Reference date matches DecayCalculator.GetDecayWeight: lastAccessed ?? created
            -- Grace: sequential|compaction=7d, workflow=14d, other=28d. Within grace → weight=1.0.
            SELECT cm.concept_name,
              MAX(
                CASE
                  WHEN julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) <=
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  THEN 1.0
                  ELSE MIN(1.0, 1.0 / sqrt(MAX(1,
                    julianday('now') - julianday(COALESCE(fc.last_accessed, fc.created, fc.last_indexed)) -
                    CASE
                      WHEN fc.file_path LIKE '%thinking/sequential/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/compaction/%' THEN 7
                      WHEN fc.file_path LIKE '%thinking/workflow/%' THEN 14
                      ELSE 28
                    END
                  )))
                END
              ) as max_decay
            FROM concept_mentions cm
            JOIN file_content fc ON cm.source_file = fc.file_path
            WHERE COALESCE(fc.last_accessed, fc.created, fc.last_indexed) IS NOT NULL
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
                PARTITION BY community_id
                ORDER BY raw_weight * max_decay DESC, concept_name ASC
              ) as rn
            FROM concept_degree WHERE raw_weight > 0
          ),
          anchors AS (
            SELECT community_id, concept_name as anchor, weighted_score as anchor_score
            FROM ranked
            WHERE rn = 1
          )
          SELECT
            r.community_id,
            r.concept_name,
            r.rn,
            a.anchor,
            a.anchor_score
          FROM ranked r
          JOIN anchors a ON a.community_id = r.community_id
          WHERE r.rn <= 10
          ORDER BY a.anchor_score DESC, r.community_id ASC, r.rn ASC, r.concept_name ASC
          `,
        )
        .all();

      const {
        communities: allCommunities,
        communityById,
        anchorByCommunityId,
      } = _buildCommunityEntries(communityRows);

      const renderedSourceBlocks: string[] = [];
      if (threadLines.length > 0) {
        renderedSourceBlocks.push(threadLines.join("\n"));
      }
      if (recentCompactionLines.length > 0) {
        renderedSourceBlocks.push(recentCompactionLines.join("\n"));
      }
      const renderedSeedLinks =
        _collectRenderedSourceSeeds(renderedSourceBlocks);

      const conceptRows = db
        .query<{ concept_name: string; community_id: number }, []>(
          `
          SELECT concept_name, community_id
          FROM concept_communities
          `,
        )
        .all();

      const conceptToAnchor = new Map<string, string>();
      for (const row of conceptRows) {
        const anchor = anchorByCommunityId.get(row.community_id);
        if (!anchor) continue;
        conceptToAnchor.set(row.concept_name.toLowerCase(), anchor);
      }

      const resolvedAnchors = new Set<string>();
      for (const link of renderedSeedLinks) {
        const anchor = conceptToAnchor.get(link.toLowerCase());
        if (anchor) resolvedAnchors.add(anchor);
      }

      let communitiesToRender: _CommunityEntry[];
      if (resolvedAnchors.size > 0) {
        communitiesToRender = allCommunities
          .filter((c) => resolvedAnchors.has(c.anchor))
          .slice(0, 10);
      } else if (renderedSourceBlocks.length > 0) {
        communitiesToRender = allCommunities.slice(0, 10);
      } else {
        const recentCommunityRows = db
          .query<_CommunityRecencyRow, []>(
            `
            SELECT
              cc.community_id,
              MAX(COALESCE(fc.last_indexed, fc.created)) AS community_updated_at
            FROM concept_communities cc
            JOIN concept_mentions cm ON cm.concept_name = cc.concept_name
            JOIN file_content fc ON fc.file_path = cm.source_file
            GROUP BY cc.community_id
            ORDER BY
              community_updated_at IS NULL ASC,
              community_updated_at DESC,
              cc.community_id ASC
            LIMIT 10
            `,
          )
          .all();

        communitiesToRender = recentCommunityRows
          .map((row) => communityById.get(row.community_id) ?? null)
          .filter(
            (community): community is _CommunityEntry => community !== null,
          );
      }

      for (const row of communitiesToRender) {
        if (row.concepts.length === 0) continue;
        const cs = row.concepts.slice(0, 10);
        priming +=
          cs.map((c, i) => (i === 0 ? `**[[${c}]]**` : `[[${c}]]`)).join(" ") +
          "\n";
      }
    }

    const lines: string[] = [
      "# maenifold",
      "",
      "Load the `maenifold` skill at session start and after every compaction. In this environment prefer using the MCP server, fall back to CLI.",
      "",
      "Prefer markdown formatting with emojis and mermaid diagrams in this environment. Keep things friendly but professional. When picking colors optimize for dark theme backgrounds.",
      "",
      "maenifold is a shared cognitive manifold. Persistent hierarchical state machines (workflows, sequential thinking sessions) execute across ephemeral agents. Each state transition produces graph artifacts (`[[WikiLinks]]`) that reshape the manifold for all future transitions by all threads — human and agent alike.",
      "",
      "- Your internal memory resets between sessions and compacts mid-session. The knowledge graph and `memory://` corpus persist across both.",
      "- `[[WikiLinks]]` in your output become graph nodes connecting concepts across sessions, agents, and domains.",
      "- Sequential thinking sessions provide structured reasoning with continuity across compaction boundaries.",
      "",
      "When thinking, reasoning, interacting with maenifold or responding tag important concepts, entities and terms as [[wikilinks]], adhering to maenifold wiki-style tagging guidelines - maenifold will construct a graph of thought based on these [[wikilinks]] connecting them across sessions and compute louvain communities from the graph.",
    ];

    if (priming) {
      lines.push(
        "",
        "Louvain communities from maenifold. Bold is the domain anchor. Traverse with `build_context`, `findsimilarconcepts` or `search_memories`.",
        "",
        "<communities>",
        priming.trimEnd(),
        "</communities>",
      );
    }

    if (threadLines.length > 0) {
      lines.push(
        "",
        "Recent activity in maenifold. Q is intent, A is conclusion. Use `read_memory` with a session ID for the full transcript.",
        "",
        "<recent-activity>",
        ...threadLines,
        "</recent-activity>",
      );
    }

    if (recentCompactionLines.length > 0) {
      lines.push(
        "",
        "Recent summaries and checkpoints in maenifold. Each entry includes memory URI and full persisted body.",
        "",
        "<recent-summaries>",
        ...recentCompactionLines,
        "</recent-summaries>",
      );
    }

    return lines.join("\n");
  } catch (err) {
    console.error(`[${_SERVICE}] _buildGraphContext FAILED: ${err}`);
    return null;
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

  const agentsMdPath = _getAgentsMdPath();
  if (!agentsMdPath) {
    console.error(`[${_SERVICE}] _refreshAgentsMd FAILED: HOME is unset`);
    return false;
  }

  const block = `${_GUARD_BEGIN}\n${context}\n${_GUARD_END}`;

  let existing = "";
  try {
    existing = await Bun.file(agentsMdPath).text();
  } catch {
    // file doesn't exist yet
  }

  const beginMatches = [...existing.matchAll(/^<!-- BEGIN:MAENIFOLD -->$/gm)];
  const endMatches = [...existing.matchAll(/^<!-- END:MAENIFOLD -->$/gm)];

  const beginIdx = beginMatches[0]?.index ?? -1;
  const endIdx =
    endMatches
      .map((m) => m.index ?? -1)
      .filter((idx) => idx > beginIdx)
      .pop() ?? -1;

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

  await Bun.write(agentsMdPath, updated);
  return true;
}

// ---------------------------------------------------------------------------
// Compaction persistence
// ---------------------------------------------------------------------------

async function _persistCompaction(
  client: any,
  sessionID: string,
): Promise<void> {
  const result = await client.session.messages({ path: { id: sessionID } });
  const messages: any[] = Array.isArray(result) ? result : (result?.data ?? []);

  const summaryMsg = [...messages]
    .reverse()
    .find((m) => m?.info?.role === "assistant" && m?.info?.summary === true);

  const raw = summaryMsg ? _extractTextParts(summaryMsg.parts) : "";
  if (!raw) {
    throw new Error(
      `No summary message found after compaction (sessionID=${sessionID})`,
    );
  }

  const sanitized = _sanitizeForStorage(raw);
  const capped = _capText(
    sanitized,
    _SUMMARY_MAX_CHARS,
    `[TRUNCATED at ${_SUMMARY_MAX_CHARS} chars]`,
  );

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

  if (survivingCount === 0) {
    throw new Error(
      `No WikiLinks survived filtering (${strippedTerms.length} stripped: ${strippedTerms.join(", ")})`,
    );
  }

  const response = [
    "## OpenCode compaction summary (untrusted)",
    "",
    `Raw Session ID: ${sessionID}`,
    `Session Slug: ${_slugSessionId(sessionID)}`,
    "",
    cleanedText,
  ].join("\n");

  const now = new Date();
  const yyyy = now.getFullYear();
  const mm = String(now.getMonth() + 1).padStart(2, "0");
  const dd = String(now.getDate()).padStart(2, "0");
  const sessionTokenHex = _encodeSessionTokenHex(sessionID);
  const payload = {
    title: `compaction sidhex ${sessionTokenHex} ts ${Date.now()}`,
    content: response,
    folder: `thinking/compaction/${yyyy}/${mm}/${dd}`,
  };

  const payloadJson = JSON.stringify(payload);
  if (payloadJson.length > _PAYLOAD_MAX_CHARS) {
    throw new Error(
      `Payload too large (${payloadJson.length} chars, max ${_PAYLOAD_MAX_CHARS})`,
    );
  }

  const proc = Bun.spawn(
    ["maenifold", "--tool", "WriteMemory", "--payload", payloadJson, "--json"],
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

  const { uri } = _parseMaenifoldOutput(stdout);

  await client.app.log({
    body: {
      service: _SERVICE,
      level: "info",
      message: "Compaction persisted",
      extra: { sessionID, uri },
    },
  });
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

export const PersistencePlugin = Object.assign(
  (async ({ client }) => {
    // Seed AGENTS.md with current graph context on startup
    if (_DEBUG)
      console.error(
        `[${_SERVICE}] init: MAENIFOLD_ROOT=${_getMaenifoldRoot() ?? "(unset)"}, DB_PATH=${_getDbPath() ?? "(unset)"}`,
      );
    try {
      const wrote = await _refreshAgentsMd();
      if (_DEBUG)
        console.error(
          `[${_SERVICE}] _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED (returned false)"}`,
        );
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
            if (_DEBUG)
              console.error(
                `[${_SERVICE}] session.created _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`,
              );
          } catch (err) {
            console.error(
              `[${_SERVICE}] session.created _refreshAgentsMd FAILED: ${err}`,
            );
          }
          return;
        }

        // --- session.updated: debounced refresh ---
        if (event.type === "session.updated") {
          if (_refreshTimer) clearTimeout(_refreshTimer);
          _refreshTimer = setTimeout(async () => {
            try {
              const wrote = await _refreshAgentsMd();
              if (_DEBUG)
                console.error(
                  `[${_SERVICE}] session.updated _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`,
                );
            } catch (err) {
              console.error(
                `[${_SERVICE}] session.updated _refreshAgentsMd FAILED: ${err}`,
              );
            }
          }, _REFRESH_DEBOUNCE_MS);
          return;
        }

        // --- session.compacted: persist summary, then refresh ---
        if (event.type === "session.compacted") {
          const sessionID = (event.properties as any)?.sessionID;
          if (_DEBUG)
            console.error(
              `[${_SERVICE}] session.compacted: sessionID=${sessionID}`,
            );
          if (typeof sessionID !== "string") {
            console.error(
              `[${_SERVICE}] session.compacted event missing sessionID`,
            );
            await client.tui.showToast({
              body: {
                title: "Persistence",
                message: "Compaction event missing sessionID",
                variant: "error",
                duration: 8000,
              },
            });
            return;
          }

          // 1. Persist compaction summary
          try {
            await _persistCompaction(client, sessionID);
            if (_DEBUG) console.error(`[${_SERVICE}] _persistCompaction: OK`);
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
            return;
          }

          // 2. Refresh AGENTS.md with fresh graph data (includes persisted summary)
          try {
            const wrote = await _refreshAgentsMd();
            if (_DEBUG)
              console.error(
                `[${_SERVICE}] post-compaction _refreshAgentsMd: ${wrote ? "WROTE" : "SKIPPED"}`,
              );
          } catch (err) {
            console.error(
              `[${_SERVICE}] post-compaction _refreshAgentsMd FAILED: ${err}`,
            );
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
  }) as Plugin,
  {
    __test__: {
      _getDefaultRoot,
      _getMaenifoldRoot,
      _getDbPath,
      _getAgentsMdPath,
      _openDb,
      _isRecentActivityEnabled,
      _isRecentCompactionsEnabled,
    },
  },
);
