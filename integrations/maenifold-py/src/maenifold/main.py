"""
Main entry point for maenifold CLI and MCP server.

Supports dual-mode operation:
- CLI mode: Direct tool invocation via command-line arguments
- MCP mode: Model Context Protocol server with stdio transport
"""

import argparse
import json
import sys
from typing import Optional

from maenifold.config import Config
from maenifold.tools.tool_registry import ToolRegistry


def _json_error(code: str, message: str) -> str:
    """Format a structured JSON error response matching C# JsonToolResponse.Fail()."""
    return json.dumps({"success": False, "error": {"code": code, "message": message}})


def print_usage():
    """Print usage information for CLI mode."""
    print("maenifold CLI")
    print()
    print("Usage:")
    print("  MCP Server mode:   maenifold --mcp")
    print("  Direct tool mode:  maenifold --tool <name> --payload '<json>' [--json]")
    print()
    print("  (For development from source: python -m maenifold.main --tool <name> --payload '<json>')")
    print()
    print("Options:")
    print("  --json             Output structured JSON instead of markdown (FR-8.1)")
    print()
    print("Examples:")
    print('  maenifold --tool WriteMemory --payload \'{"title":"Test","content":"Content with [[WikiLink]]"}\'')
    print('  maenifold --tool SearchMemories --payload \'{"query":"authentication","mode":"Hybrid"}\'')
    print('  maenifold --tool BuildContext --payload \'{"conceptName":"Machine Learning","depth":2}\'')
    print()
    print("Available tools:")
    print("  Memory: WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory")
    print("          SearchMemories, ExtractConceptsFromFile, RecentActivity")
    print("  Vector: FindSimilarConcepts")
    print("  Graph:  Sync, BuildContext, Visualize")
    print("  Think:  SequentialThinking")
    print("  Flow:   Workflow")
    print("  System: MemoryStatus, ListMemories, GetConfig, GetHelp")
    print("  Sync:   StartWatcher, StopWatcher")
    print("  Repair: RepairConcepts, AnalyzeConceptCorruption")
    print("  MCP:    ListAssets, ReadMcpResource")
    print("  Bench:  RunFullBenchmark")


def main(args: Optional[list[str]] = None):
    """
    Main entry point for maenifold.

    Args:
        args: Command-line arguments (defaults to sys.argv[1:])

    Returns:
        Exit code (0 for success, non-zero for errors)
    """
    if args is None:
        args = sys.argv[1:]

    # Parse arguments
    parser = argparse.ArgumentParser(
        description="maenifold CLI - Knowledge graph and memory system",
        add_help=False,  # We'll handle help manually
    )
    parser.add_argument("--mcp", action="store_true", help="Start MCP server mode")
    parser.add_argument("--tool", type=str, help="Tool name to invoke")
    parser.add_argument("--payload", type=str, help="JSON payload for tool")
    parser.add_argument("--json", action="store_true", help="Output structured JSON instead of markdown")
    parser.add_argument("-h", "--help", action="store_true", help="Show help message")

    try:
        parsed_args = parser.parse_args(args)
    except SystemExit:
        return 1

    # Handle help
    if parsed_args.help or not args:
        print_usage()
        return 0

    # Handle MCP server mode
    if parsed_args.mcp:
        from maenifold.mcp_server import main as mcp_main

        # Ensure required directories exist
        Config.ensure_directories()

        # Run MCP server
        mcp_main()
        return 0

    # Handle CLI tool mode
    if parsed_args.tool:
        if not parsed_args.payload:
            print("ERROR: --payload required when using --tool")
            print()
            print_usage()
            return 1

        payload_arg = parsed_args.payload
        use_json_output = parsed_args.json

        # Support payload file/STDIN to avoid shell escaping issues with large inputs
        if payload_arg.startswith("@"):
            payload_source = payload_arg[1:]
            try:
                if payload_source == "-":
                    payload_arg = sys.stdin.read()
                else:
                    with open(payload_source, "r", encoding="utf-8") as f:
                        payload_arg = f.read()
            except OSError as exc:
                print(f"Error reading payload from {payload_source or 'stdin'}: {exc}")
                return 1

        # Ensure required directories exist
        Config.ensure_directories()

        try:
            # Validate JSON before invoking
            json.loads(payload_arg)
        except json.JSONDecodeError as ex:
            if use_json_output:
                print(_json_error("INVALID_PAYLOAD", f"Error invoking {parsed_args.tool}: {ex}"))
            else:
                print(f"Error invoking {parsed_args.tool}: {ex}")
            return 0

        # Invoke the tool
        success, result = ToolRegistry.try_invoke(parsed_args.tool, payload_arg)

        if not success:
            if use_json_output:
                print(_json_error("UNKNOWN_TOOL", f"Unknown tool: {parsed_args.tool}"))
            else:
                print(f"Unknown tool: {parsed_args.tool}")
            return 0

        # Print result
        if result is not None:
            output = str(result) if not isinstance(result, str) else result
            print(output)

        return 0

    # No recognized mode
    print("ERROR: Must specify either --mcp or --tool")
    print()
    print_usage()
    return 1


if __name__ == "__main__":
    sys.exit(main())
