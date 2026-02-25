"""
Tool registry for CLI invocation.

Provides case-insensitive tool lookup and invocation from JSON payloads.
Matches C# ToolRegistry pattern for dual-mode operation (CLI and MCP).
"""

import json
from typing import Any, Callable, Optional

from maenifold.tools.memory_tools import (
    write_memory,
    read_memory,
    edit_memory,
    delete_memory,
    move_memory,
    extract_concepts_from_file,
)
from maenifold.tools.search_tools import search_memories, find_similar_concepts
from maenifold.tools.graph_tools import build_context, visualize
from maenifold.tools.concept_sync import sync
from maenifold.tools.sequential_thinking_tools import sequential_thinking
from maenifold.tools.workflow_tools import workflow
from maenifold.tools.system_tools import (
    memory_status,
    get_config,
    get_help,
    list_memories,
    update_assets,
)
from maenifold.tools.asset_tools import adopt, list_assets, read_mcp_resource
from maenifold.tools.assumption_ledger_tools import assumption_ledger
from maenifold.tools.recent_activity_tools import recent_activity
from maenifold.tools.repair_tools import analyze_concept_corruption, repair_concepts


class ToolDescriptor:
    """Describes a tool with its name, invoke function, aliases, and description."""

    def __init__(
        self,
        name: str,
        invoke: Callable[[dict], Any],
        aliases: Optional[list[str]] = None,
        description: Optional[str] = None,
    ):
        self.name = name
        self.invoke = invoke
        self.aliases = aliases or []
        self.description = description or ""


class ToolRegistry:
    """
    Registry for tool lookup and invocation.

    Provides case-insensitive tool name resolution and JSON payload parsing.
    Supports tool aliases for backward compatibility.
    """

    _by_name: dict[str, ToolDescriptor] = {}
    _initialized: bool = False

    @classmethod
    def initialize(cls) -> None:
        """Register all available tools."""
        if cls._initialized:
            return
        cls._initialized = True

        def add(descriptor: ToolDescriptor) -> None:
            """Add a tool descriptor with all its aliases."""
            cls._by_name[descriptor.name.lower()] = descriptor
            for alias in descriptor.aliases:
                cls._by_name[alias.lower()] = descriptor

        # Memory tools
        add(cls._create_write_memory_descriptor())
        add(cls._create_read_memory_descriptor())
        add(cls._create_edit_memory_descriptor())
        add(cls._create_delete_memory_descriptor())
        add(cls._create_move_memory_descriptor())
        add(cls._create_extract_concepts_from_file_descriptor())

        # Search tools
        add(cls._create_search_memories_descriptor())
        add(cls._create_find_similar_concepts_descriptor())

        # Graph tools
        add(cls._create_sync_descriptor())
        add(cls._create_build_context_descriptor())
        add(cls._create_visualize_descriptor())

        # Thinking tools
        add(cls._create_sequential_thinking_descriptor())
        add(cls._create_workflow_descriptor())

        # System tools
        add(cls._create_memory_status_descriptor())
        add(cls._create_get_config_descriptor())
        add(cls._create_get_help_descriptor())
        add(cls._create_list_memories_descriptor())
        add(cls._create_update_assets_descriptor())

        # Asset tools
        add(cls._create_adopt_descriptor())
        add(cls._create_list_assets_descriptor())
        add(cls._create_read_mcp_resource_descriptor())

        # Placeholder tools (not yet implemented in Python)
        add(cls._create_recent_activity_descriptor())
        add(cls._create_repair_concepts_descriptor())
        add(cls._create_analyze_concept_corruption_descriptor())
        add(cls._create_start_watcher_descriptor())
        add(cls._create_stop_watcher_descriptor())
        add(cls._create_run_full_benchmark_descriptor())
        add(cls._create_assumption_ledger_descriptor())

    @staticmethod
    def _create_write_memory_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            title = payload["title"]
            content = payload["content"]
            folder = payload.get("folder")
            tags = payload.get("tags")
            learn = payload.get("learn", False)
            return write_memory(title, content, folder, tags, learn)

        return ToolDescriptor("WriteMemory", invoke, ["writememory"], "Create a memory file")

    @staticmethod
    def _create_read_memory_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            identifier = payload["identifier"]
            include_checksum = payload.get("includeChecksum", True)
            learn = payload.get("learn", False)
            return read_memory(identifier, include_checksum, learn)

        return ToolDescriptor("ReadMemory", invoke, ["readmemory"], "Read a memory file")

    @staticmethod
    def _create_edit_memory_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            identifier = payload["identifier"]
            operation = payload["operation"]
            content = payload["content"]
            checksum = payload.get("checksum")
            find_text = payload.get("findText")
            section_name = payload.get("sectionName")
            expected_count = payload.get("expectedCount")
            learn = payload.get("learn", False)
            return edit_memory(
                identifier,
                operation,
                content,
                checksum,
                find_text,
                section_name,
                expected_count,
                learn,
            )

        return ToolDescriptor("EditMemory", invoke, ["editmemory"], "Edit a memory file")

    @staticmethod
    def _create_delete_memory_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            identifier = payload["identifier"]
            confirm = payload.get("confirm", False)
            learn = payload.get("learn", False)
            return delete_memory(identifier, confirm, learn)

        return ToolDescriptor("DeleteMemory", invoke, ["deletememory"], "Delete a memory file")

    @staticmethod
    def _create_move_memory_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            source = payload["source"]
            destination = payload["destination"]
            learn = payload.get("learn", False)
            return move_memory(source, destination, learn)

        return ToolDescriptor("MoveMemory", invoke, ["movememory"], "Move/rename a memory file")

    @staticmethod
    def _create_extract_concepts_from_file_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            identifier = payload["identifier"]
            learn = payload.get("learn", False)
            return extract_concepts_from_file(identifier, learn)

        return ToolDescriptor(
            "ExtractConceptsFromFile",
            invoke,
            ["extractconceptsfromfile"],
            "Extract wiki link concepts from a file",
        )

    @staticmethod
    def _create_search_memories_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            query = payload["query"]
            mode = payload.get("mode", "Hybrid")
            page_size = payload.get("pageSize", 10)
            page = payload.get("page", 1)
            folder = payload.get("folder")
            tags = payload.get("tags")
            min_score = payload.get("minScore", 0.0)
            learn = payload.get("learn", False)
            return search_memories(query, mode, page_size, page, folder, tags, min_score, learn)

        return ToolDescriptor("SearchMemories", invoke, ["searchmemories"], "Search memories")

    @staticmethod
    def _create_find_similar_concepts_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            concept_name = payload.get("conceptName", "")
            max_results = payload.get("maxResults", 10)
            learn = payload.get("learn", False)
            return find_similar_concepts(concept_name, max_results, learn)

        return ToolDescriptor(
            "FindSimilarConcepts", invoke, ["findsimilarconcepts"], "Find similar concepts"
        )

    @staticmethod
    def _create_sync_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            learn = payload.get("learn", False)
            return sync(learn)

        return ToolDescriptor("Sync", invoke, ["sync"], "Sync graph")

    @staticmethod
    def _create_build_context_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            concept_name = payload["conceptName"]
            depth = payload.get("depth", 2)
            max_entities = payload.get("maxEntities", 20)
            include_content = payload.get("includeContent", False)
            return build_context(concept_name, depth, max_entities, include_content)

        return ToolDescriptor(
            "BuildContext", invoke, ["buildcontext"], "Build context around a concept"
        )

    @staticmethod
    def _create_visualize_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            concept_name = payload["conceptName"]
            depth = payload.get("depth", 2)
            max_nodes = payload.get("maxNodes", 30)
            learn = payload.get("learn", False)
            return visualize(concept_name, depth, max_nodes, learn)

        return ToolDescriptor("Visualize", invoke, ["visualize"], "Visualize concept graph")

    @staticmethod
    def _create_sequential_thinking_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            response = payload.get("response")
            next_thought_needed = payload.get("nextThoughtNeeded", False)
            thought_number = payload.get("thoughtNumber", 0)
            total_thoughts = payload.get("totalThoughts", 0)
            session_id = payload.get("sessionId")
            cancel = payload.get("cancel", False)
            thoughts = payload.get("thoughts")
            is_revision = payload.get("isRevision", False)
            revises_thought = payload.get("revisesThought")
            branch_from_thought = payload.get("branchFromThought")
            branch_id = payload.get("branchId")
            needs_more_thoughts = payload.get("needsMoreThoughts", False)
            analysis_type = payload.get("analysisType")
            parent_workflow_id = payload.get("parentWorkflowId")
            conclusion = payload.get("conclusion")
            learn = payload.get("learn", False)

            return sequential_thinking(
                response,
                next_thought_needed,
                thought_number,
                total_thoughts,
                session_id,
                cancel,
                thoughts,
                is_revision,
                revises_thought,
                branch_from_thought,
                branch_id,
                needs_more_thoughts,
                analysis_type,
                parent_workflow_id,
                conclusion,
                learn,
            )

        return ToolDescriptor(
            "SequentialThinking", invoke, ["sequentialthinking"], "Sequential thinking tool"
        )

    @staticmethod
    def _create_workflow_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            session_id = payload.get("sessionId")
            workflow_id = payload.get("workflowId")
            response = payload.get("response")
            thoughts = payload.get("thoughts")
            status = payload.get("status")
            conclusion = payload.get("conclusion")
            view = payload.get("view", False)
            append = payload.get("append")
            learn = payload.get("learn", False)

            return workflow(
                session_id, workflow_id, response, thoughts, status, conclusion, view, append, learn
            )

        return ToolDescriptor("Workflow", invoke, ["workflow"], "Workflow tool")

    @staticmethod
    def _create_memory_status_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            learn = payload.get("learn", False)
            return memory_status(learn)

        return ToolDescriptor("MemoryStatus", invoke, ["memorystatus"], "Memory status")

    @staticmethod
    def _create_get_config_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            learn = payload.get("learn", False)
            return get_config(learn)

        return ToolDescriptor("GetConfig", invoke, ["getconfig"], "Get config")

    @staticmethod
    def _create_get_help_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            tool_name = payload.get("toolName", "")
            return get_help(tool_name)

        return ToolDescriptor("GetHelp", invoke, ["gethelp"], "Get help for a tool")

    @staticmethod
    def _create_list_memories_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            path = payload.get("path")
            learn = payload.get("learn", False)
            return list_memories(path, learn)

        return ToolDescriptor("ListMemories", invoke, ["listmemories"], "List memories")

    @staticmethod
    def _create_update_assets_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            dry_run = payload.get("dryRun", True)
            learn = payload.get("learn", False)
            return update_assets(dry_run, learn)

        return ToolDescriptor(
            "UpdateAssets", invoke, ["updateassets"], "Update assets from package"
        )

    @staticmethod
    def _create_adopt_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            type_ = payload["type"]
            identifier = payload["identifier"]
            learn = payload.get("learn", False)
            return adopt(type_, identifier, learn)

        return ToolDescriptor("Adopt", invoke, ["adopt"], "Adopt assets")

    @staticmethod
    def _create_list_assets_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            type_ = payload.get("type")
            return list_assets(type_)

        return ToolDescriptor(
            "ListAssets",
            invoke,
            ["listassets"],
            "List available asset types or metadata for a type",
        )

    @staticmethod
    def _create_read_mcp_resource_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            uri = payload["uri"]
            return read_mcp_resource(uri)

        return ToolDescriptor(
            "ReadMcpResource", invoke, ["readmcpresource"], "Read MCP resource by URI"
        )

    # Fully implemented tools below

    @staticmethod
    def _create_recent_activity_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            limit = payload.get("limit", 10)
            filter_type = payload.get("filter")
            timespan = payload.get("timespan")
            include_content = payload.get("includeContent", False)
            learn = payload.get("learn", False)
            return recent_activity(limit, filter_type, timespan, include_content, learn)

        return ToolDescriptor("RecentActivity", invoke, ["recentactivity"], "Recent activity")

    @staticmethod
    def _create_repair_concepts_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            concepts_to_replace = payload["conceptsToReplace"]
            canonical_concept = payload["canonicalConcept"]
            folder = payload.get("folder")
            dry_run = payload.get("dryRun", True)
            create_wikilinks = payload.get("createWikiLinks", False)
            min_semantic_similarity = payload.get("minSemanticSimilarity", 0.7)
            learn = payload.get("learn", False)
            return repair_concepts(
                concepts_to_replace,
                canonical_concept,
                folder,
                dry_run,
                create_wikilinks,
                min_semantic_similarity,
                learn,
            )

        return ToolDescriptor("RepairConcepts", invoke, ["repairconcepts"], "Repair concepts")

    @staticmethod
    def _create_analyze_concept_corruption_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            concept_family = payload["conceptFamily"]
            max_results = payload.get("maxResults", 50)
            learn = payload.get("learn", False)
            return analyze_concept_corruption(concept_family, max_results, learn)

        return ToolDescriptor(
            "AnalyzeConceptCorruption",
            invoke,
            ["analyzeconceptcorruption"],
            "Analyze concept corruption",
        )

    @staticmethod
    def _create_start_watcher_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> str:
            return "StartWatcher: Not yet implemented in Python port"

        return ToolDescriptor(
            "StartWatcher", invoke, ["startwatcher"], "Start incremental sync watcher"
        )

    @staticmethod
    def _create_stop_watcher_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> str:
            return "StopWatcher: Not yet implemented in Python port"

        return ToolDescriptor(
            "StopWatcher", invoke, ["stopwatcher"], "Stop incremental sync watcher"
        )

    @staticmethod
    def _create_run_full_benchmark_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> str:
            return "RunFullBenchmark: Not yet implemented in Python port"

        return ToolDescriptor(
            "RunFullBenchmark", invoke, ["runfullbenchmark"], "Run full benchmark"
        )

    @staticmethod
    def _create_assumption_ledger_descriptor() -> ToolDescriptor:
        def invoke(payload: dict) -> Any:
            action = payload["action"]
            assumption_text = payload.get("assumption")
            context = payload.get("context")
            validation_plan = payload.get("validationPlan")
            confidence = payload.get("confidence")
            concepts = payload.get("concepts")
            uri = payload.get("uri")
            status = payload.get("status")
            notes = payload.get("notes")
            learn = payload.get("learn", False)

            return assumption_ledger(
                action,
                assumption_text,
                context,
                validation_plan,
                confidence,
                concepts,
                uri,
                status,
                notes,
                learn,
            )

        return ToolDescriptor(
            "AssumptionLedger", invoke, ["assumptionledger"], "Assumption ledger"
        )

    @classmethod
    def try_invoke(cls, name: str, payload_json: str) -> tuple[bool, Optional[Any]]:
        """
        Attempt to invoke a tool by name with JSON payload.

        Args:
            name: Tool name (case-insensitive)
            payload_json: JSON string containing tool parameters

        Returns:
            Tuple of (success, result) where success is True if tool was found
        """
        if not cls._initialized:
            cls.initialize()

        if not name or not name.strip():
            return False, None

        descriptor = cls._by_name.get(name.lower())
        if descriptor is None:
            return False, None

        try:
            payload = json.loads(payload_json)
            result = descriptor.invoke(payload)
            return True, result
        except KeyError as e:
            return True, f"Error invoking {name}: Required property {e} is missing from payload."
        except Exception as e:
            return True, f"Error invoking {name}: {str(e)}"

    @classmethod
    def get_tool(cls, name: str) -> Optional[Callable[[dict], Any]]:
        """
        Get a tool's invoke function by name.

        Args:
            name: Tool name (case-insensitive)

        Returns:
            Tool invoke function or None if not found
        """
        if not cls._initialized:
            cls.initialize()

        descriptor = cls._by_name.get(name.lower())
        return descriptor.invoke if descriptor else None

    @classmethod
    def list_tools(cls) -> list[str]:
        """
        Get list of all available tool names.

        Returns:
            Sorted list of tool names (canonical names only, not aliases)
        """
        if not cls._initialized:
            cls.initialize()

        # Return only canonical names (stored with original casing)
        seen = set()
        tools = []
        for descriptor in cls._by_name.values():
            if descriptor.name not in seen:
                seen.add(descriptor.name)
                tools.append(descriptor.name)

        return sorted(tools)
