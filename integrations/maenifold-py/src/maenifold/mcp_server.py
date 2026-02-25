"""
MCP Server for maenifold knowledge graph and memory system.

Exposes all 27 maenifold tools through Model Context Protocol (MCP) using FastMCP.
Provides stdio transport for LLM integration with Claude and other MCP-compatible clients.

Tool Categories:
- Memory: WriteMemory, ReadMemory, EditMemory, DeleteMemory, MoveMemory, ExtractConceptsFromFile
- Graph: Sync, BuildContext, Visualize
- Search: SearchMemories, FindSimilarConcepts
- Session: SequentialThinking, Workflow, RecentActivity, AssumptionLedger
- System: MemoryStatus, GetConfig, GetHelp, ListMemories, UpdateAssets
- Repair: RepairConcepts, AnalyzeConceptCorruption
- Assets: Adopt, ListAssets, ReadMcpResource
- Watcher: StartAssetWatcher, StopAssetWatcher
- Benchmark: RunFullBenchmark
"""

import asyncio
import json
from typing import Any, Optional

from mcp.server.fastmcp import FastMCP

from maenifold.config import Config
from maenifold.tools.asset_tools import adopt, list_assets, read_mcp_resource
from maenifold.tools.concept_sync import sync
from maenifold.tools.graph_tools import build_context, visualize
from maenifold.tools.memory_tools import (
    delete_memory,
    edit_memory,
    extract_concepts_from_file,
    move_memory,
    read_memory,
    write_memory,
)
from maenifold.tools.search_tools import find_similar_concepts, search_memories
from maenifold.tools.sequential_thinking_tools import sequential_thinking
from maenifold.tools.recent_activity_tools import recent_activity
from maenifold.tools.system_tools import (
    get_config,
    get_help,
    list_memories,
    memory_status,
    update_assets,
)
from maenifold.tools.workflow_tools import workflow

# Initialize FastMCP server
mcp = FastMCP("maenifold")


# ============================================================================
# MEMORY TOOLS
# ============================================================================


@mcp.tool()
async def write_memory_tool(
    title: str,
    content: str,
    folder: Optional[str] = None,
    tags: Optional[list[str]] = None,
    learn: bool = False,
) -> str:
    """Creates new knowledge files with [[WikiLinks]] that automatically integrate into maenifold's graph database.
Select when AI needs to persist new learning, research findings, or structured knowledge for future retrieval.
Requires title, content with [[concepts]], optional folder organization and tag categorization.
Connects to SearchMemories for discovery, Sync for graph updates, BuildContext for relationship mapping.
Returns memory:// URI for future reference, checksum for safe editing, confirms graph integration."""
    return write_memory(title=title, content=content, folder=folder, tags=tags, learn=learn)


@mcp.tool()
async def read_memory_tool(
    identifier: str, include_checksum: bool = True, learn: bool = False
) -> str:
    """Retrieves existing knowledge files by URI or title with full content and metadata from maenifold storage.
Select when AI needs to access specific stored knowledge, verify information, or examine file structure.
Requires memory:// URI or file title, optional checksum inclusion for edit safety validation.
Integrates with SearchMemories for discovery, EditMemory for modifications, MoveMemory for organization.
Returns formatted content with timestamps, location, checksum, and full markdown content ready for analysis."""
    return read_memory(identifier=identifier, include_checksum=include_checksum, learn=learn)


@mcp.tool()
async def edit_memory_tool(
    identifier: str,
    operation: str,
    content: str,
    checksum: Optional[str] = None,
    find_text: Optional[str] = None,
    section_name: Optional[str] = None,
    expected_count: Optional[int] = None,
    learn: bool = False,
) -> str:
    """Modifies existing knowledge files with [[WikiLink]] preservation and checksum safety for graph integrity.
Select when AI needs to update, append, or restructure existing knowledge while maintaining connections.
Requires identifier, operation type, new content with [[concepts]], optional checksum and search patterns.
Connects to ReadMemory for current state, Sync for graph updates, SearchMemories for impact analysis.
Returns updated URI with new checksum, confirms successful modification and continued graph connectivity.

WARNING: find_replace uses simple string replacement. Replacing text INSIDE existing [[WikiLinks]] creates nested brackets.
Example: [[machine learning]] with findText='machine learning' and content='[[ML]]' becomes [[[[ML]]]].
To avoid this, use findText='[[machine learning]]' to replace the entire WikiLink instead."""
    return edit_memory(
        identifier=identifier,
        operation=operation,
        content=content,
        checksum=checksum,
        find_text=find_text,
        section_name=section_name,
        expected_count=expected_count,
        learn=learn,
    )


@mcp.tool()
async def delete_memory_tool(identifier: str, confirm: bool = False, learn: bool = False) -> str:
    """Permanently removes knowledge files from maenifold storage with confirmation safety mechanism.
Select when AI needs to clean up outdated, duplicate, or incorrect knowledge that impacts accuracy.
Requires file identifier and explicit confirm=true parameter to prevent accidental data loss.
Integrates with SearchMemories for cleanup discovery, ReadMemory for content verification before deletion.
Returns deletion confirmation with removed URI, enables cleanup of orphaned references."""
    return delete_memory(identifier=identifier, confirm=confirm, learn=learn)


@mcp.tool()
async def move_memory_tool(source: str, destination: str, learn: bool = False) -> str:
    """Relocates and renames knowledge files while preserving [[WikiLinks]] and updating metadata timestamps.
Select when AI needs to reorganize knowledge structure, rename files, or move content between folders.
Requires source identifier and destination path, handles both simple renaming and folder restructuring.
Connects to ReadMemory for source verification, SearchMemories for organization planning, ListMemories for structure.
Returns movement confirmation showing old → new URIs, maintains all content and graph connections."""
    return move_memory(source=source, destination=destination, learn=learn)


@mcp.tool()
async def extract_concepts_from_file_tool(identifier: str, learn: bool = False) -> str:
    """Analyzes knowledge files to identify [[WikiLink]] concepts for graph relationship discovery and validation.
Select when AI needs to understand file connectivity, validate graph integration, or analyze concept density.
Requires memory file identifier (URI or title) for targeted concept analysis.
Connects to ReadMemory for file access, BuildContext for relationship mapping, Sync for graph validation.
Returns concept list with count statistics, enables graph connectivity analysis and relationship discovery."""
    return extract_concepts_from_file(identifier=identifier, learn=learn)


# ============================================================================
# GRAPH TOOLS
# ============================================================================


@mcp.tool()
async def sync_tool(learn: bool = False) -> str:
    """Synchronizes [[WikiLink]] concepts from memory files into graph database for relationship mapping and discovery.
Select when AI needs to ensure graph accuracy, integrate new concepts, or refresh relationship data.
No parameters required - processes all memory files for comprehensive graph database synchronization.
Integrates with all memory tools for concept extraction, BuildContext for relationship verification, Visualize for graph display.
Returns synchronization status with concept counts, relationship updates, and database health confirmation."""
    return sync(learn=learn)


@mcp.tool()
async def build_context_tool(
    concept_name: str,
    depth: int = 2,
    max_entities: int = 20,
    include_content: bool = False,
    learn: bool = False,
) -> str:
    """Traverses concept relationships in maenifold's graph database to discover connected knowledge networks.
Select when AI needs to understand concept relationships, explore knowledge clusters, or find related information.
Requires concept name with optional depth, entity limits, and content inclusion for comprehensive context building.
Connects to SearchMemories for concept discovery, ReadMemory for content access, Visualize for relationship mapping.
Returns related concepts with relationship types, file references, and connection strengths for knowledge exploration."""
    result = build_context(
        concept_name=concept_name,
        depth=depth,
        max_entities=max_entities,
        include_content=include_content,
    )
    return json.dumps(result, indent=2)


@mcp.tool()
async def visualize_tool(
    concept_name: str, depth: int = 2, max_nodes: int = 30, learn: bool = False
) -> str:
    """Generates Mermaid diagram representations of concept relationships from maenifold's graph database.
Select when AI needs visual understanding of knowledge connections, relationship mapping, or concept clustering analysis.
Requires concept name with optional depth limits and node count controls for focused visualization.
Integrates with BuildContext for relationship data, Sync for graph accuracy, SearchMemories for concept discovery.
Returns Mermaid diagram code ready for rendering, enables visual knowledge architecture understanding."""
    return visualize(
        concept_name=concept_name, depth=depth, max_nodes=max_nodes, learn=learn
    )


# ============================================================================
# SEARCH TOOLS
# ============================================================================


@mcp.tool()
async def search_memories_tool(
    query: str,
    mode: str = "Hybrid",
    page_size: int = 10,
    page: int = 1,
    folder: Optional[str] = None,
    tags: Optional[list[str]] = None,
    min_score: float = 0.0,
    learn: bool = False,
) -> str:
    """Discovers existing knowledge files through flexible search modes across maenifold's memory system.
Select when AI needs to find related information, verify existing knowledge, or explore concept connections.
Supports three search modes: Hybrid (default - best of both), Semantic (concept similarity), FullText (exact matching).
Returns ranked results with detailed scoring (text, semantic, fused) to help AI understand relevance.
Integrates with ReadMemory for content access, BuildContext for relationship exploration, WriteMemory for knowledge building."""
    return search_memories(
        query=query,
        mode=mode,
        page_size=page_size,
        page=page,
        folder=folder,
        tags=tags,
        min_score=min_score,
        learn=learn,
    )


@mcp.tool()
async def find_similar_concepts_tool(
    concept_name: str, max_results: int = 10, learn: bool = False
) -> str:
    """Find concepts similar to input using vector embeddings for semantic similarity discovery.
Select when AI needs to discover related concepts based on semantic meaning rather than co-occurrence patterns.
Uses 384-dimensional embeddings with cosine similarity to find conceptually related terms.
Integrates with BuildContext for relationship exploration and SearchMemories for related file discovery.
Returns ranked concepts by semantic similarity score for knowledge graph exploration."""
    return find_similar_concepts(
        concept_name=concept_name, max_results=max_results, learn=learn
    )


# ============================================================================
# SESSION TOOLS
# ============================================================================


@mcp.tool()
async def sequential_thinking_tool(
    response: Optional[str] = None,
    next_thought_needed: bool = False,
    thought_number: int = 0,
    total_thoughts: int = 0,
    session_id: Optional[str] = None,
    cancel: bool = False,
    thoughts: Optional[str] = None,
    is_revision: bool = False,
    revises_thought: Optional[int] = None,
    branch_from_thought: Optional[int] = None,
    branch_id: Optional[str] = None,
    needs_more_thoughts: bool = False,
    analysis_type: Optional[str] = None,
    parent_workflow_id: Optional[str] = None,
    conclusion: Optional[str] = None,
    learn: bool = False,
) -> str:
    """Creates structured thinking sessions with [[concept]] integration and persistent markdown file storage.
Requires response with [[concepts]], thought tracking, session management, and optional revision capabilities.
Integrates with WriteMemory for session persistence and maenifold tools.
Returns session management with continuation guidance and checkpoint suggestions."""
    return sequential_thinking(
        response=response,
        next_thought_needed=next_thought_needed,
        thought_number=thought_number,
        total_thoughts=total_thoughts,
        session_id=session_id,
        cancel=cancel,
        thoughts=thoughts,
        is_revision=is_revision,
        revises_thought=revises_thought,
        branch_from_thought=branch_from_thought,
        branch_id=branch_id,
        needs_more_thoughts=needs_more_thoughts,
        analysis_type=analysis_type,
        parent_workflow_id=parent_workflow_id,
        conclusion=conclusion,
        learn=learn,
    )


@mcp.tool()
async def workflow_tool(
    workflow_id: Optional[str] = None,
    session_id: Optional[str] = None,
    response: Optional[str] = None,
    thoughts: Optional[str] = None,
    status: Optional[str] = None,
    conclusion: Optional[str] = None,
    append: Optional[str] = None,
    view: bool = False,
    learn: bool = False,
) -> str:
    """Orchestrates systematic problem-solving through predefined methodologies with embedded sequential thinking and tool coordination.
Use ListAssets or ReadMcpResource with asset://catalog to discover available workflows before starting.
Select when AI needs structured approaches, methodology frameworks, multi-step processes, or coordinated tool usage.
Requires workflow selection, session management, response integration, and systematic progression through defined steps.
Connects to SequentialThinking for embedded analysis, all maenifold tools for orchestrated execution.
Returns structured guidance with step progression, tool hints, quality gates, and systematic methodology execution."""
    return workflow(
        workflow_id=workflow_id,
        session_id=session_id,
        response=response,
        thoughts=thoughts,
        status=status,
        conclusion=conclusion,
        append=append,
        view=view,
        learn=learn,
    )


@mcp.tool()
async def recent_activity_tool(
    limit: int = 10,
    filter: Optional[str] = None,
    timespan: Optional[str] = None,
    include_content: bool = False,
    learn: bool = False,
) -> str:
    """Monitors maenifold system activity with time-based filtering for memory and thinking session tracking.
Select when AI needs to understand recent work patterns, find current sessions, or analyze usage trends.
Requires optional filters for activity type, time span, and result limits for focused monitoring.
Integrates with ReadMemory for session access, SearchMemories for activity analysis, MemoryStatus for system health.
Returns chronological activity list with timestamps, types, and identifiers for session continuation or analysis."""
    return recent_activity(
        limit=limit, filter=filter, timespan=timespan, include_content=include_content, learn=learn
    )


@mcp.tool()
async def assumption_ledger_tool(
    action: str,
    uri: Optional[str] = None,
    assumption: Optional[str] = None,
    concepts: Optional[list[str]] = None,
    confidence: Optional[str] = None,
    context: Optional[str] = None,
    validation_plan: Optional[str] = None,
    status: Optional[str] = None,
    notes: Optional[str] = None,
    learn: bool = False,
) -> str:
    """Declare, update, or query assumptions with minimal structure.
Captures agent reasoning without auto-inference or scoring.
Integrates with SequentialThinking and Workflow sessions via context references.
Returns declarative next-step suggestions aligned with Ma Protocol restraint."""
    # Import the function here to avoid circular imports
    from maenifold.tools.assumption_ledger_tools import assumption_ledger

    return assumption_ledger(
        action=action,
        uri=uri,
        assumption=assumption,
        concepts=concepts,
        confidence=confidence,
        context=context,
        validation_plan=validation_plan,
        status=status,
        notes=notes,
        learn=learn,
    )


# ============================================================================
# SYSTEM TOOLS
# ============================================================================


@mcp.tool()
async def memory_status_tool(learn: bool = False) -> str:
    """Provides maenifold system statistics including file counts, graph metrics, and storage health monitoring.
Select when AI needs system overview, storage analysis, or health checks before major operations.
No parameters required - returns comprehensive system state and resource utilization metrics.
Integrates with ListMemories for structure analysis, RecentActivity for usage patterns, Sync for graph health.
Returns detailed statistics with file counts, concept metrics, and system health indicators."""
    return memory_status(learn=learn)


@mcp.tool()
async def get_config_tool(learn: bool = False) -> str:
    """Displays maenifold system configuration settings and operational parameters for troubleshooting.
Select when AI needs system configuration verification, debugging support, or operational parameter analysis.
No parameters required - returns complete configuration state and system settings.
Integrates with all tools for configuration-dependent behavior, MemoryStatus for system health correlation.
Returns configuration settings with values, paths, and system state information for operational insight."""
    return get_config(learn=learn)


@mcp.tool()
async def get_help_tool(tool_name: str, learn: bool = False) -> str:
    """Retrieves comprehensive tool documentation from maenifold's help file system for detailed usage guidance.
Select when AI needs complete parameter documentation, usage examples, or troubleshooting information for any tool.
Requires tool name parameter to load specific help file from /src/assets/usage/tools/{toolname}.md.
Integrates with all maenifold tools by providing detailed usage patterns, JSON examples, and troubleshooting guidance.
Returns complete tool manual with parameters, examples, common patterns, and troubleshooting guidance."""
    return get_help(tool_name=tool_name)


@mcp.tool()
async def list_memories_tool(path: Optional[str] = None, learn: bool = False) -> str:
    """Explores memory system folder structures with file counts and sizes for knowledge organization analysis.
Select when AI needs to understand knowledge organization, plan file placement, or analyze storage patterns.
Requires optional path parameter for targeted directory exploration, defaults to memory root structure.
Connects to WriteMemory for organization planning, MoveMemory for restructuring, SearchMemories for content discovery.
Returns hierarchical directory listing with file counts, sizes, and folder organization for structure understanding."""
    return list_memories(path=path, learn=learn)


@mcp.tool()
async def update_assets_tool(dry_run: bool = True, learn: bool = False) -> str:
    """Updates persistent assets from packaged assets on upgrades with dry-run capability.
Select when asset changes (workflows, documentation, models) need refreshing after maenifold upgrades.
Supports dry-run mode to preview changes before applying, with detailed summary of modifications.
Integrates with asset initialization to provide explicit refresh mechanism after deployment.
Returns summary of added/updated files with error reporting if refresh encounters issues."""
    return update_assets(dry_run=dry_run, learn=learn)


# ============================================================================
# REPAIR TOOLS
# ============================================================================


@mcp.tool()
async def repair_concepts_tool(
    concepts_to_replace: str,
    canonical_concept: str,
    dry_run: bool = True,
    folder: Optional[str] = None,
    create_wikilinks: bool = False,
    min_semantic_similarity: float = 0.7,
    learn: bool = False,
) -> str:
    """⚠️ DANGER: This tool can PERMANENTLY DAMAGE the knowledge graph if used incorrectly!

Repairs graph corruption by replacing concept variants with canonical form in source markdown files.
Fixes the source of truth rather than just the database. Use this to consolidate concept families.

SEMANTIC VALIDATION: Automatically blocks unsafe consolidations by checking semantic similarity between concepts. Only proceeds if ALL variants have similarity >= minSemanticSimilarity threshold (default 0.7).

CRITICAL WARNINGS:
- NEVER consolidate class names (VectorTools, GraphTools) with generic concepts ([[tool]])
- NEVER merge distinct entity types (test-agent is NOT the same as [[agent]])
- ALWAYS understand the semantic meaning of concepts before consolidation
- ALWAYS use AnalyzeConceptCorruption first to understand what you're changing
- ALWAYS use dryRun=true first to review changes

SAFE Examples:
- Replace [[tools]] (plural) with [[tool]] (singular)
- Replace [[MCP tools]], [[MCP Tools]] with consistent casing

DANGEROUS Examples (DO NOT DO):
- Replacing [[VectorTools]] (a class name) with [[tool]] (destroys semantic meaning)
- Replacing [[coding-agent]] (specific agent type) with [[agent]] (loses specificity)

Test with dryRun=true first to see what would be changed."""
    # Import the function here to avoid circular imports
    from maenifold.tools.repair_tools import repair_concepts

    return repair_concepts(
        concepts_to_replace=concepts_to_replace,
        canonical_concept=canonical_concept,
        dry_run=dry_run,
        folder=folder,
        create_wikilinks=create_wikilinks,
        min_semantic_similarity=min_semantic_similarity,
        learn=learn,
    )


@mcp.tool()
async def analyze_concept_corruption_tool(
    concept_family: str, max_results: int = 50, learn: bool = False
) -> str:
    """⚠️ MUST USE BEFORE RepairConcepts! Analyzes concept corruption patterns to identify what needs repair.
Shows concept families and their variants to help plan SAFE repair operations.

CRITICAL: This tool helps you UNDERSTAND concepts before changing them:
- Identifies TRUE duplicates (plurals, case variations) that are safe to merge
- Shows class names and entity types that MUST NOT be merged with generic concepts
- Reveals the semantic structure of your knowledge graph

INTERPRETATION GUIDE:
- File Paths (e.g., GraphTools.cs): These are CLASS NAMES - DO NOT merge with [[tool]]
- Compound forms (e.g., coding-agent): These are SPECIFIC TYPES - DO NOT merge with [[agent]]
- Plurals (e.g., tools): Usually safe to merge with singular form
- Case variants (e.g., MCP tools vs MCP Tools): Safe to standardize

Always analyze BEFORE repair to avoid destroying semantic meaning!"""
    # Import the function here to avoid circular imports
    from maenifold.tools.repair_tools import analyze_concept_corruption

    return analyze_concept_corruption(
        concept_family=concept_family, max_results=max_results, learn=learn
    )


# ============================================================================
# ASSET TOOLS
# ============================================================================


@mcp.tool()
async def adopt_tool(type: str, identifier: str, learn: bool = False) -> str:
    """Adopt a role, color, or perspective by reading its JSON configuration from assets. Use ListAssets or ReadMcpResource with asset://catalog to discover available roles, colors, and perspectives before calling this tool."""
    return adopt(type=type, identifier=identifier, learn=learn)


@mcp.tool()
async def list_assets_tool(type: Optional[str] = None, learn: bool = False) -> str:
    """Lists available asset types or the metadata for a specific asset type"""
    return list_assets(type=type)


@mcp.tool()
async def read_mcp_resource_tool(uri: str, learn: bool = False) -> str:
    """Reads MCP resource content by URI (CLI-accessible)"""
    return read_mcp_resource(uri=uri)


# ============================================================================
# WATCHER TOOLS
# ============================================================================


@mcp.tool()
async def start_asset_watcher_tool(learn: bool = False) -> str:
    """Start watching assets directory for JSON file changes with debouncing"""
    # Import the function here to avoid circular imports
    from maenifold.tools.watcher_tools import start_asset_watcher

    return start_asset_watcher()


@mcp.tool()
async def stop_asset_watcher_tool(learn: bool = False) -> str:
    """Stop watching assets directory"""
    # Import the function here to avoid circular imports
    from maenifold.tools.watcher_tools import stop_asset_watcher

    return stop_asset_watcher()


# ============================================================================
# BENCHMARK TOOLS
# ============================================================================


@mcp.tool()
async def run_full_benchmark_tool(
    iterations: int = 5,
    max_test_files: int = 1000,
    include_deep_traversal: bool = True,
    learn: bool = False,
) -> str:
    """Benchmarks all maenifold performance claims including GRPH-009 CTE vs N+1, search performance, sync timing, and complex traversal bottlenecks"""
    # Import the function here to avoid circular imports
    from maenifold.tools.benchmark_tools import run_full_benchmark

    return run_full_benchmark(
        iterations=iterations,
        max_test_files=max_test_files,
        include_deep_traversal=include_deep_traversal,
        learn=learn,
    )


# ============================================================================
# MCP RESOURCES
# ============================================================================


@mcp.resource("asset://catalog")
async def get_asset_catalog() -> str:
    """Get the full asset catalog"""
    return read_mcp_resource(uri="asset://catalog")


@mcp.resource("asset://workflows/{workflow_id}")
async def get_workflow_resource(workflow_id: str) -> str:
    """Get a workflow by ID"""
    return read_mcp_resource(uri=f"asset://workflows/{workflow_id}")


@mcp.resource("asset://roles/{role_id}")
async def get_role_resource(role_id: str) -> str:
    """Get a role by ID"""
    return read_mcp_resource(uri=f"asset://roles/{role_id}")


@mcp.resource("asset://colors/{color_id}")
async def get_color_resource(color_id: str) -> str:
    """Get a color by ID"""
    return read_mcp_resource(uri=f"asset://colors/{color_id}")


@mcp.resource("asset://perspectives/{perspective_id}")
async def get_perspective_resource(perspective_id: str) -> str:
    """Get a perspective by ID"""
    return read_mcp_resource(uri=f"asset://perspectives/{perspective_id}")


# ============================================================================
# MAIN ENTRY POINT
# ============================================================================


def main():
    """Run the MCP server"""
    # Ensure required directories exist before starting
    Config.ensure_directories()

    # Run the FastMCP server
    mcp.run()


if __name__ == "__main__":
    main()
