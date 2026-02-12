# Sprint Specifications: sprint-20260212-date-detection

## Summary

Fix session path date detection bug in MarkdownWriter.GetSessionPath and related timestamp formatting issues across sequential thinking and workflow tools.

## Functional Requirements

### FR-1: GetSessionPath SHALL parse timestamp from known prefix position (T-DATE-001.1, T-DATE-001.2)

**File:** `src/Utils/MarkdownWriter.cs` lines 49-55
**Symbol:** `MarkdownWriter.GetSessionPath`

**Current (broken):**
```csharp
var timestamp = long.Parse(sessionId.AsSpan(sessionId.LastIndexOf('-') + 1), CultureInfo.InvariantCulture);
```

**Root cause:** Session IDs have format `session-{unix_ms}-{random5}` (two dashes). `LastIndexOf('-')` grabs the random suffix (e.g., `79869`), not the timestamp (`1770910549160`). Result: all sequential sessions stored under `1970/01/01/`.

**Fix approach:** Strip the known prefix (`session-`, `workflow-`), then parse the timestamp segment (everything before the next `-`, or the entire remainder if no more dashes).

**Acceptance criteria:**
- `session-1770910549160-79869` → path contains `2026/02/12/`
- `workflow-1770910549160` → path contains `2026/02/12/`
- No files created under `1970/` directory

### FR-2: IsValidSessionIdFormat SHALL use prefix-aware parsing (T-DATE-001.1)

**File:** `src/Tools/SequentialThinkingTools.cs` lines 182-190
**Symbol:** `SequentialThinkingTools.IsValidSessionIdFormat`

**Current (latent defect):** Same `LastIndexOf('-')` pattern. Accidentally works because random suffix is also numeric.

**Fix approach:** Align validation logic with the same prefix-aware timestamp extraction used in GetSessionPath. Validate that the timestamp segment (not the random suffix) is a valid long.

**Acceptance criteria:**
- `session-1770910549160-79869` → returns `true` (validates timestamp `1770910549160`)
- `session-abc-12345` → returns `false` (non-numeric timestamp segment)
- `workflow-1770910549160` → returns `true`

### FR-3: ExtractSessionId SHALL return session ID from filename (T-DATE-001 downstream)

**File:** `src/Tools/RecentActivity.Reader.cs` lines 116-122
**Symbol:** `RecentActivityReader.ExtractSessionId`

**Current (broken):** Returns `segments[1]` which is the year from the date directory path (e.g., `"2026"`) instead of the session ID.

**Fix approach:** Use `Path.GetFileNameWithoutExtension(segments[^1])` to get the filename from the last segment.

**Acceptance criteria:**
- Path `sequential/2026/02/12/session-123-456.md` → returns `session-123-456`
- Path `workflow/2026/02/12/workflow-123.md` → returns `workflow-123`

## Non-Functional Requirements

### NFR-1: Human-readable timestamps SHALL include " UTC" suffix (T-DATE-001.3)

**Locations (4 sites, verified with serena):**

| # | File | Line | Symbol |
|---|------|------|--------|
| 1 | `src/Tools/SequentialThinkingTools.cs` | 245 | `BuildThoughtSection` |
| 2 | `src/Tools/WorkflowOperations.Core.cs` | 51 | `Start` |
| 3 | `src/Tools/WorkflowOperations.Management.cs` | 65 | `Append` |
| 4 | `src/Tools/WorkflowOperations.Management.cs` | 160 | `BuildResponseContent` |

**Fix:** Change format from `"yyyy-MM-dd HH:mm:ss"` to `"yyyy-MM-dd HH:mm:ss' UTC'"`.

**Acceptance criteria:**
- All human-readable timestamps in session files end with ` UTC`
- Example: `*2026-02-12 17:30:00 UTC*`

### NFR-2: ISO 8601 timestamps SHALL use CultureInfo.InvariantCulture (T-DATE-001.4)

**Locations (2 sites, verified with serena):**

| # | File | Line | Symbol |
|---|------|------|--------|
| 1 | `src/Tools/SequentialThinkingTools.cs` | 299 | `FinalizeSession` (cancel) |
| 2 | `src/Tools/SequentialThinkingTools.cs` | 319 | `FinalizeSession` (complete) |

**Fix:** Change `DateTime.UtcNow.ToString("o")` to `DateTime.UtcNow.ToString("o", CultureInfo.InvariantCulture)`.

**Acceptance criteria:**
- Frontmatter `cancelled` and `completed` fields use InvariantCulture
- Consistent with WorkflowOperations which already does this correctly (lines 177, 189, 207)

## Implementation Constraints

- **Technology:** .NET 9, C#
- **Dependencies:** No new dependencies
- **Files modified (6 files total):**
  1. `src/Utils/MarkdownWriter.cs` — GetSessionPath fix
  2. `src/Tools/SequentialThinkingTools.cs` — IsValidSessionIdFormat, BuildThoughtSection, FinalizeSession
  3. `src/Tools/WorkflowOperations.Core.cs` — Start timestamp
  4. `src/Tools/WorkflowOperations.Management.cs` — Append, BuildResponseContent timestamps
  5. `src/Tools/RecentActivity.Reader.cs` — ExtractSessionId fix
  6. `tests/Maenifold.Tests/` — New test file(s) for regression coverage

## MUST NOT HAVE

- No backward compatibility shims for old `1970/01/01/` paths
- No scope creep beyond the 6 changes listed above
- No refactoring of unrelated code
- No changes to SessionCleanup.cs or SystemTools.cs (adjacent scope, separate sprint)
