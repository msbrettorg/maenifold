"""
System tools for maenifold.

Provides:
- MemoryStatus: System statistics and health monitoring
- GetConfig: Configuration display
- GetHelp: Tool documentation retrieval
- ListMemories: Directory structure exploration
- UpdateAssets: Asset synchronization (stub)
"""

import os
import sqlite3
import hashlib
import shutil
from pathlib import Path
from typing import Optional
from urllib.parse import unquote

from maenifold.config import Config
from maenifold.utils.sqlite_extensions import open_read_only, query_single


def memory_status(learn: bool = False) -> str:
    """
    Provides maenifold system statistics including file counts, graph metrics,
    and storage health monitoring.

    Select when AI needs system overview, storage analysis, or health checks
    before major operations. No parameters required - returns comprehensive
    system state and resource utilization metrics.

    Integrates with ListMemories for structure analysis, RecentActivity for
    usage patterns, Sync for graph health. Returns detailed statistics with
    file counts, concept metrics, and system health indicators.

    Args:
        learn: Return help documentation instead of executing

    Returns:
        Formatted status string with file counts, database metrics, and sizes
    """
    if learn:
        return _get_learn_content("MemoryStatus")

    stats = ["# Memory System Status\n"]

    # Count files and folders
    files = []
    folders = []
    for root, dirs, filenames in os.walk(Config.memory_path):
        folders.extend([os.path.join(root, d) for d in dirs])
        files.extend([os.path.join(root, f) for f in filenames if f.endswith(".md")])

    stats.append(f"### Files\n- Total files: {len(files)}")
    stats.append(f"- Total folders: {len(folders)}")

    # Calculate total size
    total_size = sum(os.path.getsize(f) for f in files)
    stats.append(f"- Total size: {total_size / 1024.0 / 1024.0:.2f} MB")

    # Database statistics
    if os.path.exists(Config.database_path):
        try:
            with open_read_only(Config.database_path) as conn:
                concept_count = query_single(conn, "SELECT COUNT(*) FROM concepts") or 0
                relation_count = query_single(conn, "SELECT COUNT(*) FROM concept_graph") or 0
                mention_count = query_single(conn, "SELECT COUNT(*) FROM concept_mentions") or 0

                stats.append("\n### Graph Database")
                stats.append(f"- Concepts: {concept_count}")
                stats.append(f"- Relations: {relation_count}")
                stats.append(f"- Mentions: {mention_count}")

                db_size = os.path.getsize(Config.database_path)
                stats.append(f"- Database size: {db_size / 1024.0 / 1024.0:.2f} MB")
        except Exception:
            stats.append("\n### Graph Database\n- Error reading database (run sync first)")
    else:
        stats.append("\n### Graph Database\n- Not initialized (run sync first)")

    return "\n".join(stats)


def get_config(learn: bool = False) -> str:
    """
    Displays maenifold system configuration settings and operational parameters
    for troubleshooting.

    Select when AI needs system configuration verification, debugging support,
    or operational parameter analysis. No parameters required - returns complete
    configuration state and system settings.

    Integrates with all tools for configuration-dependent behavior, MemoryStatus
    for system health correlation. Returns configuration settings with values,
    paths, and system state information for operational insight.

    Args:
        learn: Return help documentation instead of executing

    Returns:
        Configuration summary string
    """
    if learn:
        return _get_learn_content("GetConfig")

    return Config.get_config_summary()


def get_help(tool_name: str) -> str:
    """
    Retrieves comprehensive tool documentation from maenifold's help file system
    for detailed usage guidance.

    Select when AI needs complete parameter documentation, usage examples, or
    troubleshooting information for any tool. Requires tool name parameter to
    load specific help file from /src/assets/usage/tools/{toolname}.md.

    Integrates with all maenifold tools by providing detailed usage patterns,
    JSON examples, and troubleshooting guidance. Returns complete tool manual
    with parameters, examples, common patterns, and troubleshooting guidance.

    Args:
        tool_name: Tool name (e.g., 'WriteMemory', 'SearchMemories')

    Returns:
        Tool documentation content or error message with available tools
    """
    help_path = os.path.join(
        Config.assets_path,
        "usage",
        "tools",
        f"{tool_name.lower()}.md"
    )

    if not os.path.exists(help_path):
        # List available tools
        tools_dir = os.path.join(Config.assets_path, "usage", "tools")
        if os.path.exists(tools_dir):
            available = sorted([
                os.path.splitext(f)[0]
                for f in os.listdir(tools_dir)
                if f.endswith(".md")
            ])
            tools_list = "\n".join(f"  - {t}" for t in available)
        else:
            tools_list = "  (tools directory not found)"

        return (
            f"No help file found for tool: {tool_name}\n\n"
            f"Available tools with help:\n{tools_list}"
        )

    with open(help_path, "r", encoding="utf-8") as f:
        return f.read()


def _validate_path_security(path: str) -> Optional[str]:
    """
    Validates that a path is safe and doesn't contain traversal attempts.

    This function prevents path traversal attacks by:
    1. Rejecting absolute paths (both Unix and Windows style)
    2. URL-decoding the path recursively to catch multi-encoded traversal attempts
    3. Checking for '.' and '..' components in normalized path segments

    Args:
        path: The path to validate (should be relative)

    Returns:
        Error message if validation fails, None if path is safe
    """
    if not path:
        return None

    # Reject Unix absolute paths
    if os.path.isabs(path):
        return "Absolute paths are not allowed. Use relative paths only."

    # RT-SEC-001: Reject Windows absolute paths (C:\ style) on any platform
    # This is important for cross-platform security
    if len(path) >= 2 and path[1] == ':':
        return "Absolute paths are not allowed. Use relative paths only."

    # RT-SEC-001: URL-decode recursively to prevent encoded traversal bypass
    # This catches %2e%2e%2f (../) and double-encoded %252e%252e%252f
    decoded_path = path
    max_iterations = 5  # Prevent infinite loop from malicious input
    for _ in range(max_iterations):
        new_decoded = unquote(decoded_path)
        if new_decoded == decoded_path:
            break  # No more decoding needed
        decoded_path = new_decoded

    # Normalize path separators
    normalized = decoded_path.replace('\\', '/')

    # Split and check each segment for traversal components
    segments = [s for s in normalized.split('/') if s]  # Remove empty segments
    for segment in segments:
        if segment in ('.', '..'):
            return "Path traversal components ('.' and '..') are not allowed."

    return None


def list_memories(path: Optional[str] = None, learn: bool = False) -> str:
    """
    Explores memory system folder structures with file counts and sizes for
    knowledge organization analysis.

    Select when AI needs to understand knowledge organization, plan file
    placement, or analyze storage patterns. Requires optional path parameter
    for targeted directory exploration, defaults to memory root structure.

    Connects to WriteMemory for organization planning, MoveMemory for
    restructuring, SearchMemories for content discovery. Returns hierarchical
    directory listing with file counts, sizes, and folder organization for
    structure understanding.

    Args:
        path: Directory path (relative to memory)
        learn: Return help documentation instead of executing

    Returns:
        Formatted directory listing with folders and files
    """
    if learn:
        return _get_learn_content("ListMemories")

    # RT-SEC-001: Validate path security before any filesystem operations
    if path:
        validation_error = _validate_path_security(path)
        if validation_error:
            return f"ERROR: Invalid path - {validation_error}"

    # Resolve target path
    target_path = Config.memory_path if not path else os.path.join(Config.memory_path, path)

    # RT-SEC-001: Additional defense-in-depth check using realpath
    # This catches any remaining traversal attempts after validation
    target_path_real = os.path.realpath(target_path)
    memory_path_real = os.path.realpath(Config.memory_path)

    # Normalize both paths for comparison - target must be within or equal to memory
    # Ensure both paths end with separator for accurate prefix check
    if not memory_path_real.endswith(os.sep):
        memory_path_real += os.sep

    # For root directory listing (path is None), target_path_real equals memory_path_real
    # so we need to allow exact matches too
    if not (target_path_real == memory_path_real.rstrip(os.sep) or
            target_path_real.startswith(memory_path_real)):
        return f"ERROR: Path escapes memory root: {path}"

    if not os.path.exists(target_path_real):
        return f"Directory not found: {path}"

    output = [f"# Directory: {path or '/'}\n"]

    # List directories
    try:
        entries = os.listdir(target_path_real)
    except PermissionError:
        return f"Permission denied: {path}"

    dirs = sorted([e for e in entries if os.path.isdir(os.path.join(target_path_real, e))])
    files = sorted([e for e in entries if e.endswith(".md") and os.path.isfile(os.path.join(target_path_real, e))])

    if dirs:
        output.append("### Folders")
        for d in dirs:
            dir_path = os.path.join(target_path_real, d)
            file_count = len([
                f for f in os.listdir(dir_path)
                if f.endswith(".md") and os.path.isfile(os.path.join(dir_path, f))
            ]) if os.path.isdir(dir_path) else 0
            output.append(f"📁 {d}/ ({file_count} files)")
        output.append("")

    if files:
        output.append("### Files")
        for f in files:
            file_path = os.path.join(target_path_real, f)
            file_size = os.path.getsize(file_path)
            output.append(f"📄 {f} ({file_size / 1024.0:.1f} KB)")

    return "\n".join(output)


def update_assets(dry_run: bool = True, learn: bool = False) -> str:
    """
    Updates persistent assets from packaged assets on upgrades with dry-run
    capability.

    Select when asset changes (workflows, documentation, models) need refreshing
    after maenifold upgrades. Supports dry-run mode to preview changes before
    applying, with detailed summary of modifications.

    Integrates with asset initialization to provide explicit refresh mechanism
    after deployment. Returns summary of added/updated files with error reporting
    if refresh encounters issues.

    Args:
        dry_run: Preview changes without modifying files (default=True)
        learn: Return help documentation instead of executing

    Returns:
        Summary of changes (or preview if dry_run=True)
    """
    if learn:
        return _get_learn_content("UpdateAssets")

    # Determine source assets path (within the package)
    import maenifold
    package_dir = Path(maenifold.__file__).parent
    source_assets_path = package_dir / "assets"

    # Target is user's persistent location
    target_assets_path = Path(Config.assets_path)

    # Track results
    added_files = []
    updated_files = []
    errors = []

    # Validate directories
    if not source_assets_path.exists():
        return "# Asset Update Summary\n\n## Error\nSource assets directory not found"

    if not target_assets_path.exists():
        return "# Asset Update Summary\n\n## Error\nTarget assets directory does not exist. Run initialization first."

    def get_file_hash(file_path: Path) -> bytes:
        """Calculate SHA256 hash of file."""
        sha256 = hashlib.sha256()
        with open(file_path, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b''):
                sha256.update(chunk)
        return sha256.digest()

    def files_are_identical(file1: Path, file2: Path) -> bool:
        """Check if two files are identical by hash."""
        try:
            return get_file_hash(file1) == get_file_hash(file2)
        except Exception:
            return False

    def compare_and_update(source_dir: Path, target_dir: Path):
        """Recursively compare and update directories."""
        # Create target dir if needed
        if not target_dir.exists():
            target_dir.mkdir(parents=True, exist_ok=True)

        # Process files in source
        for source_file in source_dir.iterdir():
            if source_file.is_file():
                target_file = target_dir / source_file.name

                if not target_file.exists():
                    # New file
                    added_files.append(str(target_file))
                    if not dry_run:
                        try:
                            shutil.copy2(source_file, target_file)
                        except Exception as ex:
                            errors.append(f"Failed to copy new file {source_file.name}: {str(ex)}")
                else:
                    # Check if file changed
                    if source_file.stat().st_size != target_file.stat().st_size or \
                       not files_are_identical(source_file, target_file):
                        updated_files.append(str(target_file))
                        if not dry_run:
                            try:
                                shutil.copy2(source_file, target_file)
                            except Exception as ex:
                                errors.append(f"Failed to update file {source_file.name}: {str(ex)}")

            elif source_file.is_dir():
                # Recurse into subdirectory
                compare_and_update(source_file, target_dir / source_file.name)

    # Run comparison
    try:
        compare_and_update(source_assets_path, target_assets_path)
    except Exception as ex:
        return f"# Asset Update Summary\n\n## Error\nUpdate failed: {str(ex)}"

    # Format result
    result = ["# Asset Update Summary\n"]
    result.append(f"## Results\n- Added files: {len(added_files)}")
    result.append(f"- Updated files: {len(updated_files)}")
    result.append(f"- Errors: {len(errors)}")

    if added_files:
        result.append("\n### Added Files")
        for f in sorted(added_files):
            rel_path = os.path.relpath(f, Config.assets_path)
            result.append(f"- {rel_path}")

    if updated_files:
        result.append("\n### Updated Files")
        for f in sorted(updated_files):
            rel_path = os.path.relpath(f, Config.assets_path)
            result.append(f"- {rel_path}")

    if errors:
        result.append("\n### Errors")
        for error in errors:
            result.append(f"- {error}")

    if dry_run and (added_files or updated_files):
        result.append("\n*This was a dry run. Run with dryRun=false to apply changes.*")

    return "\n".join(result)


def _get_learn_content(tool_name: str) -> str:
    """
    Load learning/help content for a tool.

    Args:
        tool_name: Name of the tool

    Returns:
        Help content or placeholder message
    """
    return get_help(tool_name)
