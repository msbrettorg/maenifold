#!/usr/bin/env python3
"""Cross-implementation parity test harness: maenifold C# vs Rust.

// T-1.6: RTM NFR-1 — cross-implementation JSON parity
// T-1.6: RTM NFR-2 — functional equivalence between C# and Rust

Runs the same CLI commands against both implementations, compares JSON
outputs, and reports PASS / DRIFT / ERROR for each test case.

Reuses patterns from:
  - tests/support/cli_runner.rs   (command invocation)
  - tests/support/compat_env.rs   (isolated sandbox layout)
  - tests/support/json_semantic.rs (JSON canonicalisation / normalisation)
  - tests/parity-harness.sh       (test matrix + comparison approach)

Usage:
    python3 tests/parity/run_parity.py [TEST_ID ...]

    With no arguments, all test cases are executed.
    Specific test IDs can be passed to filter: e.g. MEM-W01 SYS-M01

Exit codes:
    0  all tests PASS
    1  one or more tests DRIFT or ERROR
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Tuple

# ─── Constants ────────────────────────────────────────────────────────────────

CSHARP_PROJECT = os.environ.get(
    "MAENIFOLD_CSHARP_PROJECT",
    "/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj",
)

# Resolve workspace root from this file's location: tests/parity/ → ../..
_HERE = os.path.dirname(os.path.abspath(__file__))
RUST_WORKSPACE = os.path.dirname(os.path.dirname(_HERE))

# Fields stripped from every comparison regardless of test-level ignore_fields.
# These are volatile/implementation-specific values that must always be excluded.
# Mirrors the exclusions in tests/parity-harness.sh normalize_json().
_GLOBAL_IGNORE = frozenset({
    "created",
    "updated",
    "modified",
    "last_indexed",
    "last_accessed",
    "timestamp",
    "created_at",
    "updated_at",
})

# Column widths for the output table
_COL_ID     = 14
_COL_STATUS = 7
_COL_DETAIL = 54
_TABLE_WIDTH = _COL_ID + 2 + _COL_STATUS + 2 + _COL_DETAIL


# ─── Data model ───────────────────────────────────────────────────────────────

@dataclass
class SetupCommand:
    """A prerequisite CLI command that must succeed on both sandboxes."""
    tool: str
    payload: str


@dataclass
class TestCase:
    """One parity test entry from the test matrix.

    compare_mode controls what equality means:
      "exact"        canonical JSON must match verbatim (after field ignoring)
      "structural"   only the shape (key names + value types) must match
      "error_match"  both must return success=false; error.code must agree
      "success_only" both must return success=true; data values are not checked
    """
    id: str
    tool: str
    payload: str
    compare_mode: str = "structural"
    ignore_fields: List[str] = field(default_factory=list)
    setup: List[SetupCommand] = field(default_factory=list)
    description: str = ""


@dataclass
class CliRun:
    """Result of a single CLI invocation."""
    exit_code: int
    stdout: str
    stderr: str

    @property
    def ok(self) -> bool:
        return self.exit_code == 0


@dataclass
class TestResult:
    """Outcome of a single parity comparison."""
    test_id: str
    status: str          # PASS | DRIFT | ERROR | SKIP
    detail: str
    csharp_out: str = ""
    rust_out: str = ""


# ─── Sandbox ──────────────────────────────────────────────────────────────────

class Sandbox:
    """Isolated filesystem environment (MAENIFOLD_ROOT + memory.db) for one
    implementation.

    C# and Rust each receive their own sandbox so their SQLite databases never
    conflict.  The same logical state is reproduced in each sandbox via shared
    setup commands before each stateful test.

    Layout mirrors tests/support/compat_env.rs IsolatedRoot:
        <root>/
            memory/           MAENIFOLD_ROOT/memory
            assets/           MAENIFOLD_ROOT/assets
            memory.db         MAENIFOLD_DATABASE_PATH
    """

    def __init__(self, label: str) -> None:
        self.root = tempfile.mkdtemp(prefix=f"maenifold-parity-{label}-")
        self.db   = os.path.join(self.root, "memory.db")
        os.makedirs(os.path.join(self.root, "memory"), exist_ok=True)
        os.makedirs(os.path.join(self.root, "assets"),  exist_ok=True)

    @property
    def env(self) -> Dict[str, str]:
        """Merged OS environment with sandbox-specific overrides."""
        e = dict(os.environ)
        e["MAENIFOLD_ROOT"]          = self.root
        e["MAENIFOLD_DATABASE_PATH"] = self.db
        return e

    def cleanup(self) -> None:
        shutil.rmtree(self.root, ignore_errors=True)

    def __repr__(self) -> str:
        return f"Sandbox({self.root!r})"


# ─── CLI runners ──────────────────────────────────────────────────────────────
# Mirror the logic in tests/support/cli_runner.rs run_csharp_cli / run_rust_cli.

def run_csharp(tool: str, payload: str, sandbox: Sandbox) -> CliRun:
    """Invoke: dotnet run --project <CSHARP_PROJECT> -- --tool T --payload P --json"""
    cmd = [
        "dotnet", "run", "--project", CSHARP_PROJECT, "--",
        "--tool", tool, "--payload", payload, "--json",
    ]
    return _invoke(cmd, sandbox.env, cwd=None)


def run_rust(tool: str, payload: str, sandbox: Sandbox) -> CliRun:
    """Invoke: cargo run -q -p maenifold-cli -- --tool T --payload P --json
    (or a pre-built binary from MAENIFOLD_RUST_CLI / CARGO_BIN_EXE_* if set)
    """
    explicit = (
        os.environ.get("MAENIFOLD_RUST_CLI")
        or os.environ.get("CARGO_BIN_EXE_maenifold-cli")
        or os.environ.get("CARGO_BIN_EXE_maenifold")
    )
    if explicit:
        cmd = [explicit, "--tool", tool, "--payload", payload, "--json"]
        return _invoke(cmd, sandbox.env, cwd=None)

    cmd = [
        "cargo", "run", "-q", "-p", "maenifold-cli", "--",
        "--tool", tool, "--payload", payload, "--json",
    ]
    return _invoke(cmd, sandbox.env, cwd=RUST_WORKSPACE)


def _invoke(
    cmd: List[str],
    env: Dict[str, str],
    cwd: Optional[str],
) -> CliRun:
    try:
        proc = subprocess.run(
            cmd,
            env=env,
            cwd=cwd,
            capture_output=True,
            text=True,
        )
        return CliRun(
            exit_code=proc.returncode,
            stdout=proc.stdout.strip(),
            stderr=proc.stderr.strip(),
        )
    except FileNotFoundError as exc:
        return CliRun(exit_code=-1, stdout="", stderr=str(exc))
    except Exception as exc:  # noqa: BLE001 — intentional broad catch for runner errors
        return CliRun(exit_code=-2, stdout="", stderr=f"subprocess error: {exc}")


# ─── JSON normalisation ───────────────────────────────────────────────────────
# Ported from tests/support/json_semantic.rs (Python block inside canonicalize()).

def _normalize(value: Any, ignored: frozenset) -> Any:
    """Recursively normalise a decoded JSON value:
    - dicts: sort keys, drop ignored fields
    - lists: sort deterministically (by id/name/json-repr)
    - floats that are integers: coerce to int
    """
    if isinstance(value, dict):
        return {
            k: _normalize(v, ignored)
            for k, v in sorted(value.items())
            if k not in ignored
        }
    if isinstance(value, float) and value.is_integer():
        return int(value)
    if isinstance(value, list):
        items = [_normalize(v, ignored) for v in value]
        if all(isinstance(i, dict) for i in items):
            def _dict_key(item: Any) -> tuple:
                return (
                    item.get("id"),
                    item.get("name"),
                    json.dumps(item, sort_keys=True, separators=(",", ":")),
                )
            return sorted(items, key=_dict_key)
        return sorted(
            items,
            key=lambda i: json.dumps(i, sort_keys=True, separators=(",", ":")),
        )
    return value


def _parse_normalize(text: str, ignored: frozenset) -> Tuple[bool, Any]:
    """Parse JSON and normalise it.  Returns (ok, value_or_error_string)."""
    try:
        raw = json.loads(text)
    except (json.JSONDecodeError, ValueError) as exc:
        return False, f"JSON parse error: {exc}"
    return True, _normalize(raw, ignored)


def _jstr(value: Any) -> str:
    return json.dumps(value, sort_keys=True, separators=(",", ":"))


# ─── Structural shape extraction ──────────────────────────────────────────────

def _shape(value: Any) -> Any:
    """Reduce a JSON value to its structural shape: key names + type names only.
    Values are discarded; only the schema is preserved.
    """
    if isinstance(value, dict):
        return {k: _shape(v) for k, v in sorted(value.items())}
    if isinstance(value, list):
        if not value:
            return {"type": "array", "length": 0}
        unique_shapes = sorted({_jstr(_shape(i)) for i in value})
        return {
            "type": "array",
            "length": len(value),
            "item_shapes": unique_shapes,
        }
    if value is None:
        return "null"
    return type(value).__name__


# ─── Path scrubbing ───────────────────────────────────────────────────────────

def _scrub(text: str, paths: List[str]) -> str:
    """Replace sandbox-specific absolute paths with <ROOT> so that outputs
    from C# and Rust (which live in different temp dirs) can be compared."""
    for p in paths:
        if p:
            text = text.replace(p, "<ROOT>")
    return text


# ─── Comparison logic ─────────────────────────────────────────────────────────

def compare_runs(
    tc: TestCase,
    cs: CliRun,
    rs: CliRun,
    sandbox_paths: List[str],
) -> TestResult:
    """Apply the test case's compare_mode and return a TestResult."""

    # subprocess-level failure (binary not found, cargo compilation error, etc.)
    if cs.exit_code < 0:
        return TestResult(tc.id, "ERROR", f"C# invoker: {cs.stderr[:120]}")
    if rs.exit_code < 0:
        return TestResult(tc.id, "ERROR", f"Rust invoker: {rs.stderr[:120]}")

    ignore = _GLOBAL_IGNORE | frozenset(tc.ignore_fields)
    cs_raw = _scrub(cs.stdout, sandbox_paths)
    rs_raw = _scrub(rs.stdout, sandbox_paths)

    # ── success_only ──────────────────────────────────────────────────────────
    if tc.compare_mode == "success_only":
        cs_ok, cs_val = _parse_normalize(cs_raw, ignore)
        rs_ok, rs_val = _parse_normalize(rs_raw, ignore)
        if not cs_ok:
            return TestResult(tc.id, "ERROR", f"C# bad JSON: {cs_raw[:80]}", cs_raw, rs_raw)
        if not rs_ok:
            return TestResult(tc.id, "ERROR", f"Rust bad JSON: {rs_raw[:80]}", cs_raw, rs_raw)
        cs_succ = isinstance(cs_val, dict) and cs_val.get("success") is True
        rs_succ = isinstance(rs_val, dict) and rs_val.get("success") is True
        if not cs_succ:
            return TestResult(tc.id, "DRIFT", "C# did not return success=true", cs_raw, rs_raw)
        if not rs_succ:
            return TestResult(tc.id, "DRIFT", "Rust did not return success=true", cs_raw, rs_raw)
        return TestResult(tc.id, "PASS", "both returned success=true")

    # ── error_match ───────────────────────────────────────────────────────────
    if tc.compare_mode == "error_match":
        cs_ok, cs_val = _parse_normalize(cs_raw, ignore)
        rs_ok, rs_val = _parse_normalize(rs_raw, ignore)
        if not cs_ok:
            return TestResult(tc.id, "ERROR", f"C# bad JSON: {cs_raw[:80]}", cs_raw, rs_raw)
        if not rs_ok:
            return TestResult(tc.id, "ERROR", f"Rust bad JSON: {rs_raw[:80]}", cs_raw, rs_raw)
        cs_succ = isinstance(cs_val, dict) and cs_val.get("success") is True
        rs_succ = isinstance(rs_val, dict) and rs_val.get("success") is True
        if cs_succ:
            return TestResult(tc.id, "DRIFT", "C# unexpectedly succeeded", cs_raw, rs_raw)
        if rs_succ:
            return TestResult(tc.id, "DRIFT", "Rust unexpectedly succeeded", cs_raw, rs_raw)
        # Both errored — error.code must agree
        cs_code = (
            cs_val.get("error", {}).get("code")
            if isinstance(cs_val, dict) else None
        )
        rs_code = (
            rs_val.get("error", {}).get("code")
            if isinstance(rs_val, dict) else None
        )
        if cs_code != rs_code:
            return TestResult(
                tc.id, "DRIFT",
                f"error.code: C#={cs_code!r} Rust={rs_code!r}",
                cs_raw, rs_raw,
            )
        return TestResult(tc.id, "PASS", f"both errored (code={cs_code!r})")

    # ── exact / structural — parse first ──────────────────────────────────────
    cs_ok, cs_val = _parse_normalize(cs_raw, ignore)
    rs_ok, rs_val = _parse_normalize(rs_raw, ignore)
    if not cs_ok:
        return TestResult(tc.id, "ERROR", f"C# bad JSON: {cs_raw[:80]}", cs_raw, rs_raw)
    if not rs_ok:
        return TestResult(tc.id, "ERROR", f"Rust bad JSON: {rs_raw[:80]}", cs_raw, rs_raw)

    # Always verify the top-level success flag agrees (envelope sanity check)
    if isinstance(cs_val, dict) and isinstance(rs_val, dict):
        cs_s = cs_val.get("success")
        rs_s = rs_val.get("success")
        if cs_s != rs_s:
            return TestResult(
                tc.id, "DRIFT",
                f"success mismatch: C#={cs_s} Rust={rs_s}",
                cs_raw, rs_raw,
            )

    # ── exact ─────────────────────────────────────────────────────────────────
    if tc.compare_mode == "exact":
        cs_str = _jstr(cs_val)
        rs_str = _jstr(rs_val)
        if cs_str == rs_str:
            return TestResult(tc.id, "PASS", "exact match")
        return TestResult(tc.id, "DRIFT", "canonical JSON differs", cs_raw, rs_raw)

    # ── structural ────────────────────────────────────────────────────────────
    cs_shape_str = _jstr(_shape(cs_val))
    rs_shape_str = _jstr(_shape(rs_val))
    if cs_shape_str == rs_shape_str:
        return TestResult(tc.id, "PASS", "structural match")
    return TestResult(tc.id, "DRIFT", "structure differs", cs_raw, rs_raw)


# ─── Test matrix (T-1.6: RTM NFR-1) ──────────────────────────────────────────
# 33 test cases across 7 categories.
# Ordering is intentional: stateless error tests first, then stateful
# write → read → edit → delete sequences.

def _sc(tool: str, payload: str) -> SetupCommand:
    """Shorthand for constructing a SetupCommand."""
    return SetupCommand(tool=tool, payload=payload)


TEST_CASES: List[TestCase] = [

    # ──────────────────────────────────────────────────────────────────────────
    # SYS-* System Tools  (no prior state required)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="SYS-M01",
        tool="MemoryStatus",
        payload="{}",
        compare_mode="structural",
        description="MemoryStatus returns status envelope",
    ),
    TestCase(
        id="SYS-C01",
        tool="GetConfig",
        payload="{}",
        compare_mode="structural",
        # Root / path fields are sandbox-specific; ignore them
        ignore_fields=["rootPath", "databasePath", "memoryRoot", "assetsPath"],
        description="GetConfig returns config key structure",
    ),
    TestCase(
        id="SYS-H01",
        tool="GetHelp",
        payload='{"toolName":"WriteMemory"}',
        compare_mode="structural",
        description="GetHelp returns help envelope for WriteMemory",
    ),
    TestCase(
        id="SYS-H02",
        tool="GetHelp",
        payload='{"toolName":"FakeToolThatDoesNotExist"}',
        compare_mode="error_match",
        description="GetHelp errors on unknown tool",
    ),
    TestCase(
        id="SYS-L01",
        tool="ListMemories",
        payload="{}",
        compare_mode="structural",
        description="ListMemories on empty sandbox root",
    ),
    TestCase(
        id="SYS-L03",
        tool="ListMemories",
        payload='{"path":"nonexistent/folder"}',
        compare_mode="structural",
        description="ListMemories handles missing folder gracefully",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-W* Write errors  (stateless — no files required)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-W04",
        tool="WriteMemory",
        payload="{}",
        compare_mode="error_match",
        description="WriteMemory: missing title → error",
    ),
    TestCase(
        id="MEM-W05",
        tool="WriteMemory",
        payload='{"title":"missing-content-test"}',
        compare_mode="error_match",
        description="WriteMemory: missing content → error",
    ),
    TestCase(
        id="MEM-W06",
        tool="WriteMemory",
        payload='{"title":"no-links","content":"No wiki links here"}',
        compare_mode="error_match",
        description="WriteMemory: content without [[WikiLinks]] → error",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-R*/D*/M* stateless errors  (target files do not need to exist)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-R04",
        tool="ReadMemory",
        payload='{"identifier":"memory://does-not-exist"}',
        compare_mode="error_match",
        description="ReadMemory: non-existent file → error",
    ),
    TestCase(
        id="MEM-D02",
        tool="DeleteMemory",
        # parity-test-1 does not exist yet at this point in test order
        payload='{"identifier":"memory://parity-test-1","confirm":false}',
        compare_mode="error_match",
        description="DeleteMemory: confirm=false → error",
    ),
    TestCase(
        id="MEM-M03",
        tool="MoveMemory",
        payload='{"source":"memory://nonexistent/does-not-exist","destination":"memory://anywhere"}',
        compare_mode="error_match",
        description="MoveMemory: non-existent source → error",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-W01  Write (seeds state consumed by subsequent read/edit/delete tests)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-W01",
        tool="WriteMemory",
        payload='{"title":"parity-test-1","content":"Testing [[parity]] between implementations"}',
        compare_mode="structural",
        ignore_fields=["checksum", "uri"],
        description="WriteMemory creates parity-test-1 in both sandboxes",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-R*  Read back  (depend on MEM-W01)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-R01",
        tool="ReadMemory",
        payload='{"identifier":"memory://parity-test-1"}',
        compare_mode="structural",
        ignore_fields=["checksum"],
        description="ReadMemory retrieves parity-test-1 by URI",
    ),
    TestCase(
        id="MEM-R02",
        tool="ReadMemory",
        payload='{"identifier":"parity-test-1"}',
        compare_mode="structural",
        ignore_fields=["checksum"],
        description="ReadMemory retrieves parity-test-1 by title",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-E*  Edit  (depend on MEM-W01)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-E01",
        tool="EditMemory",
        payload=(
            '{"identifier":"memory://parity-test-1","operation":"append",'
            '"content":"Appended [[content]]"}'
        ),
        compare_mode="structural",
        ignore_fields=["checksum"],
        description="EditMemory appends to parity-test-1",
    ),
    TestCase(
        id="MEM-E05",
        tool="EditMemory",
        # Deliberately wrong checksum (all-zeros SHA-256 look-alike)
        payload=(
            '{"identifier":"memory://parity-test-1","operation":"append",'
            '"content":"[[stale]] checksum [[test]]",'
            '"checksum":"0000000000000000000000000000000000000000000000000000000000000000"}'
        ),
        compare_mode="error_match",
        description="EditMemory: stale checksum → error",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # MEM-D01  Delete  (setup writes a dedicated file so MEM-W01 is preserved)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="MEM-D01",
        tool="DeleteMemory",
        payload='{"identifier":"memory://parity-delete-test","confirm":true}',
        compare_mode="success_only",
        setup=[
            _sc(
                "WriteMemory",
                '{"title":"parity-delete-test","content":"Delete [[this]] [[file]]"}',
            ),
        ],
        description="DeleteMemory: confirm=true deletes file",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # SRC-*  Search  (operates on whatever is written so far)
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="SRC-S01",
        tool="SearchMemories",
        payload='{"query":"parity","mode":"Hybrid"}',
        compare_mode="structural",
        ignore_fields=["score", "fusedScore", "textScore", "semanticScore", "uri", "path"],
        description="SearchMemories Hybrid mode returns result shape",
    ),
    TestCase(
        id="SRC-S02",
        tool="SearchMemories",
        payload='{"query":"parity","mode":"Semantic"}',
        compare_mode="structural",
        ignore_fields=["score", "fusedScore", "textScore", "semanticScore", "uri", "path"],
        description="SearchMemories Semantic mode returns result shape",
    ),
    TestCase(
        id="SRC-S03",
        tool="SearchMemories",
        payload='{"query":"parity","mode":"FullText"}',
        compare_mode="structural",
        ignore_fields=["score", "fusedScore", "textScore", "semanticScore", "uri", "path"],
        description="SearchMemories FullText mode returns result shape",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # GRP-*  Graph operations
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="GRP-B03",
        tool="BuildContext",
        payload='{"conceptName":"nonexistentxyz123"}',
        compare_mode="structural",
        description="BuildContext: unknown concept returns empty neighbourhood",
    ),
    TestCase(
        id="GRP-V02",
        tool="Visualize",
        payload='{"conceptName":"nonexistentconceptxyz123"}',
        compare_mode="structural",
        description="Visualize: unknown concept response shape",
    ),
    TestCase(
        id="GRP-F01",
        tool="FindSimilarConcepts",
        payload='{"conceptName":"parity"}',
        compare_mode="structural",
        ignore_fields=["score", "similarity", "conceptName"],
        description="FindSimilarConcepts returns result envelope shape",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # SES-*  Session tools
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="SES-T01",
        tool="SequentialThinking",
        payload=(
            '{"thoughtNumber":0,"response":"Testing [[parity]]",'
            '"totalThoughts":2,"nextThoughtNeeded":true}'
        ),
        compare_mode="structural",
        ignore_fields=["sessionId", "id", "workflowSessionId"],
        description="SequentialThinking starts a new session",
    ),
    TestCase(
        id="SES-A01",
        tool="RecentActivity",
        payload="{}",
        compare_mode="structural",
        ignore_fields=["uri", "id", "path", "sessionId"],
        description="RecentActivity returns activity envelope shape",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # WORK-*  Workflow orchestration
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="WORK-009",
        tool="Workflow",
        payload='{"workflowId":"nonexistent-workflow-id"}',
        compare_mode="error_match",
        description="Workflow: invalid workflow ID → error",
    ),
    TestCase(
        id="WORK-W01",
        tool="Workflow",
        payload='{"workflowId":"deductive-reasoning"}',
        compare_mode="structural",
        ignore_fields=["sessionId", "id", "workflowSessionId", "position"],
        description="Workflow: start deductive-reasoning returns step envelope",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # AST-*  Asset tools
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="AST-L01",
        tool="ListAssets",
        payload="{}",
        compare_mode="structural",
        description="ListAssets returns asset catalog structure",
    ),
    TestCase(
        id="AST-C01",
        tool="ReadMcpResource",
        payload='{"uri":"asset://catalog"}',
        compare_mode="structural",
        description="ReadMcpResource returns catalog resource envelope",
    ),

    # ──────────────────────────────────────────────────────────────────────────
    # FAIL-*  Documented failure cases
    # ──────────────────────────────────────────────────────────────────────────

    TestCase(
        id="FAIL-001",
        tool="WriteMemory",
        payload='{"title":"no-links-fail","content":"No wikilinks in this content at all"}',
        compare_mode="error_match",
        description="WriteMemory without WikiLinks (documented failure)",
    ),
    TestCase(
        id="FAIL-002",
        tool="SequentialThinking",
        payload=(
            '{"thoughtNumber":0,"response":"No wikilinks here at all",'
            '"totalThoughts":1,"nextThoughtNeeded":true}'
        ),
        compare_mode="error_match",
        description="SequentialThinking without WikiLinks (documented failure)",
    ),
    TestCase(
        id="FAIL-005",
        tool="BuildContext",
        payload='{"conceptName":"unknownconceptxyz999"}',
        compare_mode="structural",
        description="BuildContext unknown concept returns empty (not error)",
    ),
]


# ─── Runner ───────────────────────────────────────────────────────────────────

def run_setup(
    cmd: SetupCommand,
    cs_sandbox: Sandbox,
    rs_sandbox: Sandbox,
) -> Optional[str]:
    """Execute a setup command against both sandboxes.

    Returns None on success, or an error description string on failure.
    Both implementations must succeed; if either fails the calling test is
    marked ERROR and skipped.
    """
    cs_run = run_csharp(cmd.tool, cmd.payload, cs_sandbox)
    if not cs_run.ok:
        return (
            f"C# setup '{cmd.tool}' failed (exit {cs_run.exit_code}): "
            f"{cs_run.stderr[:120]}"
        )
    rs_run = run_rust(cmd.tool, cmd.payload, rs_sandbox)
    if not rs_run.ok:
        return (
            f"Rust setup '{cmd.tool}' failed (exit {rs_run.exit_code}): "
            f"{rs_run.stderr[:120]}"
        )
    return None


def run_test(
    tc: TestCase,
    cs_sandbox: Sandbox,
    rs_sandbox: Sandbox,
) -> TestResult:
    """Execute a single test case and return its parity result."""
    for setup_cmd in tc.setup:
        err = run_setup(setup_cmd, cs_sandbox, rs_sandbox)
        if err:
            return TestResult(tc.id, "ERROR", f"setup failed: {err}")

    cs_run = run_csharp(tc.tool, tc.payload, cs_sandbox)
    rs_run = run_rust(tc.tool, tc.payload, rs_sandbox)

    sandbox_paths = [cs_sandbox.root, rs_sandbox.root]
    return compare_runs(tc, cs_run, rs_run, sandbox_paths)


# ─── Output formatting ────────────────────────────────────────────────────────

_STATUS_LABEL = {
    "PASS":  "PASS ",
    "DRIFT": "DRIFT",
    "ERROR": "ERROR",
    "SKIP":  "SKIP ",
}


def _bar() -> str:
    return "─" * _TABLE_WIDTH


def _row(result: TestResult) -> str:
    status_label = _STATUS_LABEL.get(result.status, result.status[:5].ljust(5))
    detail_text  = result.detail[: _COL_DETAIL]
    return f" {result.test_id:<{_COL_ID}}  {status_label}  {detail_text}"


def _pretty_json(text: str) -> str:
    """Pretty-print text as JSON, falling back to raw text on parse failure."""
    try:
        return json.dumps(json.loads(text), indent=2)
    except (json.JSONDecodeError, ValueError):
        return text[:800]


def print_summary(results: List[TestResult]) -> None:
    """Print the summary table and, for failing tests, their detailed diffs."""
    header = (
        f" {'Test ID':<{_COL_ID}}  "
        f"{'Status':<{_COL_STATUS}}  "
        f"Details"
    )
    print(_bar())
    print(header)
    print(_bar())
    for r in results:
        print(_row(r))
    print(_bar())

    passes = sum(1 for r in results if r.status == "PASS")
    drifts = sum(1 for r in results if r.status == "DRIFT")
    errors = sum(1 for r in results if r.status == "ERROR")
    skips  = sum(1 for r in results if r.status == "SKIP")
    total  = len(results)
    print(
        f" PASS: {passes}  DRIFT: {drifts}  ERROR: {errors}  "
        f"SKIP: {skips}  Total: {total}"
    )
    print(_bar())

    # Detailed sections for every non-passing test
    failing = [r for r in results if r.status in ("DRIFT", "ERROR")]
    if not failing:
        return

    print()
    print("─" * _TABLE_WIDTH)
    print(" Drift / Error Details")
    print("─" * _TABLE_WIDTH)

    for r in failing:
        print()
        print(f"  [{r.status}] {r.test_id}  — {r.detail}")

        if r.csharp_out:
            print("  ── C# output ──")
            for line in _pretty_json(r.csharp_out).splitlines():
                print(f"    {line}")

        if r.rust_out:
            print("  ── Rust output ──")
            for line in _pretty_json(r.rust_out).splitlines():
                print(f"    {line}")

    print()


# ─── Main ─────────────────────────────────────────────────────────────────────

def main(argv: List[str]) -> int:
    filter_ids = set(argv)
    cases = (
        TEST_CASES
        if not filter_ids
        else [tc for tc in TEST_CASES if tc.id in filter_ids]
    )

    if not cases:
        print(f"No test cases matched: {filter_ids!r}", file=sys.stderr)
        return 1

    # One shared sandbox per implementation; both persist across the entire run
    # so that stateful tests can rely on state written by earlier tests.
    cs_sandbox = Sandbox("csharp")
    rs_sandbox = Sandbox("rust")

    try:
        print(f"Parity harness — {len(cases)} test(s)")
        print(f"  C# project    : {CSHARP_PROJECT}")
        print(f"  Rust workspace: {RUST_WORKSPACE}")
        print(f"  C# sandbox    : {cs_sandbox.root}")
        print(f"  Rust sandbox  : {rs_sandbox.root}")
        print()

        results: List[TestResult] = []
        for tc in cases:
            label = f"  {tc.id:<{_COL_ID}} ({tc.tool})"
            print(f"{label}… ", end="", flush=True)
            result = run_test(tc, cs_sandbox, rs_sandbox)
            results.append(result)
            print(result.status)

        print()
        print_summary(results)

        failed = any(r.status in ("DRIFT", "ERROR") for r in results)
        return 1 if failed else 0

    finally:
        cs_sandbox.cleanup()
        rs_sandbox.cleanup()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
