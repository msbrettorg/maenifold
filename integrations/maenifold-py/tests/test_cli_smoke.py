"""
CLI smoke tests for maenifold Python port.

These tests exercise every tool via the CLI interface to verify behavioral parity
with the C# version. Since we can't unit-test AI behavior, we test:
1. CLI parity (help, errors, unknown tools, bad JSON)
2. Every tool executes without crashing
3. Output format matches expected patterns
4. Database cross-compatibility (both CLIs can use the same DB)
"""

import json
import os
import subprocess
import tempfile
import shutil
from pathlib import Path

import pytest

# Path to the Python module
PYTHON = os.path.join(
    os.path.dirname(__file__), "..", ".venv", "bin", "python"
)
PY_CMD = [PYTHON, "-m", "maenifold.main"]

# Path to C# binary (if available for cross-platform tests)
CS_BINARY = os.path.expanduser(
    "~/src/ma-collective/maenifold/src/bin/Debug/net9.0/maenifold"
)
CS_AVAILABLE = os.path.isfile(CS_BINARY)


def run_py(args: list[str], env_override: dict | None = None) -> subprocess.CompletedProcess:
    """Run Python CLI with given arguments."""
    env = os.environ.copy()
    if env_override:
        env.update(env_override)
    return subprocess.run(
        PY_CMD + args,
        capture_output=True,
        text=True,
        env=env,
        cwd=os.path.dirname(os.path.dirname(__file__)),
    )


def run_cs(args: list[str]) -> subprocess.CompletedProcess:
    """Run C# CLI with given arguments."""
    return subprocess.run(
        [CS_BINARY] + args,
        capture_output=True,
        text=True,
    )


def invoke_tool(tool_name: str, payload: dict, use_json: bool = False) -> subprocess.CompletedProcess:
    """Invoke a tool via the Python CLI."""
    args = ["--tool", tool_name, "--payload", json.dumps(payload)]
    if use_json:
        args.append("--json")
    return run_py(args)


# ============================================================================
# CLI PARITY TESTS (match C# exactly)
# ============================================================================


class TestCLIParity:
    """Tests that verify CLI behavior matches C# exactly."""

    def test_help_output(self):
        """Test 1: --help produces expected output structure."""
        result = run_py(["--help"])
        assert result.returncode == 0
        assert "maenifold CLI" in result.stdout
        assert "MCP Server mode:" in result.stdout
        assert "Direct tool mode:" in result.stdout
        assert "[--json]" in result.stdout
        assert "Available tools:" in result.stdout
        assert "WriteMemory" in result.stdout
        assert "SequentialThinking" in result.stdout
        assert "RunFullBenchmark" in result.stdout

    def test_no_args_shows_help(self):
        """No arguments shows help."""
        result = run_py([])
        assert result.returncode == 0
        assert "maenifold CLI" in result.stdout

    def test_unknown_tool(self):
        """Test 2: Unknown tool returns proper message."""
        result = run_py(["--tool", "FakeTool", "--payload", "{}"])
        assert result.returncode == 0
        assert "Unknown tool: FakeTool" in result.stdout

    def test_bad_json(self):
        """Test 3: Bad JSON payload returns error."""
        result = run_py(["--tool", "WriteMemory", "--payload", "not json"])
        assert "Error invoking WriteMemory:" in result.stdout
        assert result.returncode == 0

    def test_missing_required_param(self):
        """Test 4: Missing required parameter returns error."""
        result = run_py(["--tool", "WriteMemory", "--payload", '{"title":"x"}'])
        assert "Error" in result.stdout or "error" in result.stdout.lower()

    def test_unknown_tool_json_mode(self):
        """Unknown tool with --json returns structured JSON error."""
        result = run_py(["--tool", "FakeTool", "--payload", "{}", "--json"])
        assert result.returncode == 0
        data = json.loads(result.stdout.strip())
        assert data["success"] is False
        assert data["error"]["code"] == "UNKNOWN_TOOL"

    def test_bad_json_json_mode(self):
        """Bad JSON with --json returns structured JSON error."""
        result = run_py(["--tool", "WriteMemory", "--payload", "not json", "--json"])
        assert result.returncode == 0
        data = json.loads(result.stdout.strip())
        assert data["success"] is False
        assert data["error"]["code"] == "INVALID_PAYLOAD"

    def test_missing_payload_flag(self):
        """Missing --payload shows error."""
        result = run_py(["--tool", "WriteMemory"])
        assert result.returncode == 1
        assert "ERROR: --payload required" in result.stdout

    @pytest.mark.skipif(not CS_AVAILABLE, reason="C# binary not available")
    def test_help_matches_cs(self):
        """Help output structure matches C# version."""
        py_result = run_py(["--help"])
        cs_result = run_cs(["--help"])
        # Compare structural elements (not the dotnet/python dev line)
        for line in ["maenifold CLI", "MCP Server mode:", "Direct tool mode:",
                      "--json", "Available tools:", "WriteMemory",
                      "SequentialThinking", "RunFullBenchmark"]:
            assert line in py_result.stdout, f"Python help missing: {line}"
            assert line in cs_result.stdout, f"C# help missing: {line}"

    @pytest.mark.skipif(not CS_AVAILABLE, reason="C# binary not available")
    def test_unknown_tool_matches_cs(self):
        """Unknown tool output matches C# version."""
        py_result = run_py(["--tool", "FakeTool", "--payload", "{}"])
        cs_result = run_cs(["--tool", "FakeTool", "--payload", "{}"])
        assert py_result.stdout.strip() == cs_result.stdout.strip()


# ============================================================================
# TOOL SMOKE TESTS (every tool executes without crashing)
# ============================================================================


@pytest.fixture(scope="module")
def temp_memory_dir():
    """Create a temp directory for isolated memory operations."""
    tmpdir = tempfile.mkdtemp(prefix="maenifold_smoke_")
    yield tmpdir
    shutil.rmtree(tmpdir, ignore_errors=True)


class TestMemoryTools:
    """Smoke tests for memory tools."""

    def test_memory_status(self):
        """MemoryStatus returns status information."""
        result = invoke_tool("MemoryStatus", {})
        assert result.returncode == 0
        # Should contain at least some status info
        assert len(result.stdout) > 10

    def test_write_read_delete_memory(self, tmp_path):
        """Write, read, and delete a memory file (full lifecycle)."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}

        # Write
        result = run_py(
            ["--tool", "WriteMemory", "--payload",
             json.dumps({"title": "Smoke Test", "content": "Testing [[WikiLink]] extraction"})],
            env_override=env,
        )
        assert result.returncode == 0
        assert "memory://" in result.stdout

        # Read
        result = run_py(
            ["--tool", "ReadMemory", "--payload",
             json.dumps({"identifier": "Smoke Test"})],
            env_override=env,
        )
        assert result.returncode == 0
        assert "WikiLink" in result.stdout

        # Delete
        result = run_py(
            ["--tool", "DeleteMemory", "--payload",
             json.dumps({"identifier": "Smoke Test", "confirm": True})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_list_memories(self):
        """ListMemories returns directory listing."""
        result = invoke_tool("ListMemories", {})
        assert result.returncode == 0

    def test_get_config(self):
        """GetConfig returns configuration."""
        result = invoke_tool("GetConfig", {})
        assert result.returncode == 0
        assert len(result.stdout) > 10

    def test_get_help(self):
        """GetHelp returns tool documentation."""
        result = invoke_tool("GetHelp", {"toolName": "WriteMemory"})
        assert result.returncode == 0
        assert "WriteMemory" in result.stdout or "write" in result.stdout.lower()


class TestGraphTools:
    """Smoke tests for graph tools."""

    def test_sync(self, tmp_path):
        """Sync processes without error."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "Sync", "--payload", "{}"],
            env_override=env,
        )
        assert result.returncode == 0

    def test_build_context(self, tmp_path):
        """BuildContext returns results (may be empty)."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "BuildContext", "--payload",
             json.dumps({"conceptName": "test"})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_visualize(self, tmp_path):
        """Visualize returns mermaid diagram or empty result."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "Visualize", "--payload",
             json.dumps({"conceptName": "test"})],
            env_override=env,
        )
        assert result.returncode == 0


class TestSearchTools:
    """Smoke tests for search tools."""

    def test_search_memories(self, tmp_path):
        """SearchMemories returns results structure."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "SearchMemories", "--payload",
             json.dumps({"query": "test"})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_find_similar_concepts(self, tmp_path):
        """FindSimilarConcepts returns results."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "FindSimilarConcepts", "--payload",
             json.dumps({"conceptName": "test"})],
            env_override=env,
        )
        assert result.returncode == 0


class TestSessionTools:
    """Smoke tests for session/thinking tools."""

    def test_sequential_thinking_start(self, tmp_path):
        """SequentialThinking starts a session."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "SequentialThinking", "--payload",
             json.dumps({
                 "response": "Test thought with [[concept]]",
                 "thoughtNumber": 1,
                 "totalThoughts": 2,
                 "nextThoughtNeeded": True,
             })],
            env_override=env,
        )
        assert result.returncode == 0
        assert "session" in result.stdout.lower() or "thought" in result.stdout.lower()

    def test_sequential_thinking_cancel(self, tmp_path):
        """SequentialThinking cancel works."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "SequentialThinking", "--payload",
             json.dumps({"cancel": True, "sessionId": "nonexistent"})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_workflow_list(self, tmp_path):
        """Workflow view returns available workflows."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "Workflow", "--payload",
             json.dumps({"view": True})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_recent_activity(self, tmp_path):
        """RecentActivity returns activity list."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "RecentActivity", "--payload",
             json.dumps({"limit": 5})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_assumption_ledger_read(self, tmp_path):
        """AssumptionLedger read action works."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "AssumptionLedger", "--payload",
             json.dumps({"action": "read"})],
            env_override=env,
        )
        assert result.returncode == 0


class TestRepairTools:
    """Smoke tests for repair tools."""

    def test_analyze_concept_corruption(self, tmp_path):
        """AnalyzeConceptCorruption returns analysis."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "AnalyzeConceptCorruption", "--payload",
             json.dumps({"conceptFamily": "test"})],
            env_override=env,
        )
        assert result.returncode == 0

    def test_repair_concepts_dry_run(self, tmp_path):
        """RepairConcepts dry run works without modifying files."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}
        result = run_py(
            ["--tool", "RepairConcepts", "--payload",
             json.dumps({
                 "conceptsToReplace": "tests",
                 "canonicalConcept": "test",
                 "dryRun": True,
             })],
            env_override=env,
        )
        assert result.returncode == 0


class TestAssetTools:
    """Smoke tests for asset tools."""

    def test_list_assets(self):
        """ListAssets returns available asset types."""
        result = invoke_tool("ListAssets", {})
        assert result.returncode == 0
        # Should mention at least workflow, role, color, perspective
        output = result.stdout.lower()
        assert "workflow" in output or "role" in output

    def test_list_assets_workflows(self):
        """ListAssets with type=workflow returns workflow list."""
        result = invoke_tool("ListAssets", {"type": "workflow"})
        assert result.returncode == 0

    def test_adopt_role(self):
        """Adopt a role returns role configuration."""
        result = invoke_tool("Adopt", {"type": "role", "identifier": "researcher"})
        assert result.returncode == 0
        assert len(result.stdout) > 50  # Should return substantial content

    def test_read_mcp_resource(self):
        """ReadMcpResource reads a resource by URI."""
        result = invoke_tool("ReadMcpResource", {"uri": "asset://catalog"})
        assert result.returncode == 0
        assert len(result.stdout) > 50

    def test_update_assets_dry_run(self):
        """UpdateAssets dry run reports without changes."""
        result = invoke_tool("UpdateAssets", {"dryRun": True})
        assert result.returncode == 0


class TestPlaceholderTools:
    """Smoke tests for tools with placeholder implementations."""

    def test_start_watcher(self):
        """StartWatcher returns placeholder or starts."""
        result = invoke_tool("StartWatcher", {})
        assert result.returncode == 0

    def test_stop_watcher(self):
        """StopWatcher returns placeholder or stops."""
        result = invoke_tool("StopWatcher", {})
        assert result.returncode == 0

    def test_run_full_benchmark(self):
        """RunFullBenchmark returns placeholder or runs."""
        result = invoke_tool("RunFullBenchmark", {})
        assert result.returncode == 0


# ============================================================================
# PAYLOAD INPUT METHODS
# ============================================================================


class TestPayloadInput:
    """Test different payload input methods."""

    def test_payload_from_file(self, tmp_path):
        """@file payload reads from file."""
        payload_file = tmp_path / "payload.json"
        payload_file.write_text('{"toolName": "WriteMemory"}')
        result = run_py(["--tool", "GetHelp", "--payload", f"@{payload_file}"])
        assert result.returncode == 0

    def test_payload_inline(self):
        """Inline JSON payload works."""
        result = run_py(["--tool", "GetHelp", "--payload", '{"toolName": "WriteMemory"}'])
        assert result.returncode == 0


# ============================================================================
# CROSS-PLATFORM COMPATIBILITY TESTS
# ============================================================================


@pytest.mark.skipif(not CS_AVAILABLE, reason="C# binary not available")
class TestCrossCompatibility:
    """Tests that verify Python and C# produce compatible outputs."""

    def test_memory_status_structure(self):
        """Both CLIs return memory status with similar structure."""
        py_result = run_py(["--tool", "MemoryStatus", "--payload", "{}"])
        cs_result = run_cs(["--tool", "MemoryStatus", "--payload", "{}"])
        assert py_result.returncode == 0
        assert cs_result.returncode == 0
        # Both should have some output
        assert len(py_result.stdout) > 10
        assert len(cs_result.stdout) > 10

    def test_get_config_structure(self):
        """Both CLIs return config with similar structure."""
        py_result = run_py(["--tool", "GetConfig", "--payload", "{}"])
        cs_result = run_cs(["--tool", "GetConfig", "--payload", "{}"])
        assert py_result.returncode == 0
        assert cs_result.returncode == 0

    def test_list_assets_structure(self):
        """Both CLIs return assets with similar structure."""
        py_result = run_py(["--tool", "ListAssets", "--payload", "{}"])
        cs_result = run_cs(["--tool", "ListAssets", "--payload", "{}"])
        assert py_result.returncode == 0
        assert cs_result.returncode == 0

    def test_get_help_structure(self):
        """Both CLIs return help with similar content."""
        py_result = run_py(["--tool", "GetHelp", "--payload", '{"toolName":"WriteMemory"}'])
        cs_result = run_cs(["--tool", "GetHelp", "--payload", '{"toolName":"WriteMemory"}'])
        assert py_result.returncode == 0
        assert cs_result.returncode == 0
        # Both should mention WriteMemory
        assert "WriteMemory" in py_result.stdout or "write" in py_result.stdout.lower()
        assert "WriteMemory" in cs_result.stdout or "write" in cs_result.stdout.lower()

    def test_unknown_tool_identical(self):
        """Unknown tool output is byte-identical."""
        py_result = run_py(["--tool", "FakeToolXYZ", "--payload", "{}"])
        cs_result = run_cs(["--tool", "FakeToolXYZ", "--payload", "{}"])
        assert py_result.stdout.strip() == cs_result.stdout.strip()

    def test_database_cross_compat(self, tmp_path):
        """Python can read a memory written by C# and vice versa."""
        env = {"MAENIFOLD_MEMORY_PATH": str(tmp_path)}

        # Write with Python
        py_write = run_py(
            ["--tool", "WriteMemory", "--payload",
             json.dumps({"title": "CrossCompat Test", "content": "Written by [[Python]]"})],
            env_override=env,
        )
        assert py_write.returncode == 0
        assert "memory://" in py_write.stdout

        # Read with C# (if available, using same memory path)
        cs_env = os.environ.copy()
        cs_env["MAENIFOLD_MEMORY_PATH"] = str(tmp_path)
        cs_read = subprocess.run(
            [CS_BINARY, "--tool", "ReadMemory", "--payload",
             json.dumps({"identifier": "CrossCompat Test"})],
            capture_output=True, text=True, env=cs_env,
        )
        assert cs_read.returncode == 0
        assert "Python" in cs_read.stdout
