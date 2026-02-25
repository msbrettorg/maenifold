"""
Concept repair tools for analyzing and fixing graph corruption.

Provides tools for:
- Analyzing concept families and identifying variants that need consolidation
- Repairing concept corruption by replacing variants with canonical forms
- Semantic similarity validation to prevent destructive merges
"""

import os
import re
from pathlib import Path
from typing import Dict, List, Optional, Set, Tuple

from maenifold.config import Config
from maenifold.utils.markdown_io import normalize_concept, read_markdown, write_markdown
from maenifold.utils.vector_tools import VectorTools


# WikiLink pattern
_WIKILINK_PATTERN = re.compile(r"\[\[([^\]]+)\]\]")


def cosine_similarity(vector_a: List[float], vector_b: List[float]) -> float:
    """
    Calculate cosine similarity between two vectors.

    Args:
        vector_a: First vector
        vector_b: Second vector

    Returns:
        Cosine similarity score (0.0 to 1.0)
    """
    if not vector_a or not vector_b:
        return 0.0

    # Ensure vectors are same length
    max_len = max(len(vector_a), len(vector_b))
    a = vector_a + [0.0] * (max_len - len(vector_a))
    b = vector_b + [0.0] * (max_len - len(vector_b))

    # Calculate dot product
    dot_product = sum(x * y for x, y in zip(a, b))

    # Calculate magnitudes
    magnitude_a = sum(x * x for x in a) ** 0.5
    magnitude_b = sum(x * x for x in b) ** 0.5

    if magnitude_a == 0.0 or magnitude_b == 0.0:
        return 0.0

    return dot_product / (magnitude_a * magnitude_b)


def validate_input(
    concepts_to_replace: str,
    canonical_concept: str,
    create_wikilinks: bool
) -> Tuple[Set[str], str, bool, str]:
    """
    Validate input parameters for repair operation.

    Args:
        concepts_to_replace: Comma/semicolon/pipe-separated list of variants
        canonical_concept: Canonical form to use
        create_wikilinks: Whether creating WikiLinks from plain text

    Returns:
        Tuple of (variants_set, normalized_canonical, removing_brackets, error_message)
    """
    # Split and normalize variants
    variants = set()
    for separator in [',', ';', '|']:
        if separator in concepts_to_replace:
            parts = concepts_to_replace.split(separator)
            for part in parts:
                normalized = normalize_concept(part.strip())
                if normalized:
                    variants.add(normalized)
            break
    else:
        # No separator found, treat as single concept
        normalized = normalize_concept(concepts_to_replace.strip())
        if normalized:
            variants.add(normalized)

    if not variants:
        return (set(), "", False, "ERROR: No concepts to replace provided")

    removing_brackets = not canonical_concept or not canonical_concept.strip()
    normalized_canonical = "" if removing_brackets else normalize_concept(canonical_concept)

    if create_wikilinks and removing_brackets:
        return (
            set(),
            "",
            False,
            "ERROR: Cannot both create WikiLinks and remove them (createWikiLinks=true with empty canonicalConcept)"
        )

    return (variants, normalized_canonical, removing_brackets, "")


def validate_semantic_similarity(
    variants: Set[str],
    normalized_canonical: str,
    min_semantic_similarity: float
) -> List[str]:
    """
    Validate semantic similarity between variants and canonical concept.

    Args:
        variants: Set of concept variants to check
        normalized_canonical: Normalized canonical concept
        min_semantic_similarity: Minimum similarity threshold (0.0-1.0)

    Returns:
        List of result messages (last line is "BLOCKED" if unsafe)
    """
    results = []
    results.append("=== SEMANTIC VALIDATION ===")
    results.append(f"Checking semantic similarity with threshold {min_semantic_similarity:.2f}...")

    try:
        canonical_embedding = VectorTools.generate_embedding(normalized_canonical)

        unsafe_variants = []
        for variant in variants:
            variant_embedding = VectorTools.generate_embedding(variant)
            similarity = cosine_similarity(variant_embedding, canonical_embedding)

            if similarity < min_semantic_similarity:
                unsafe_variants.append((variant, similarity))

        if unsafe_variants:
            results.append("")
            results.append("WARNING: Unsafe consolidation detected! The following concepts are semantically dissimilar:")
            for variant, similarity in sorted(unsafe_variants, key=lambda x: x[1]):
                results.append(f"  - {variant}: similarity {similarity:.3f}")
            results.append("")
            results.append("This would destroy semantic meaning. Review concepts manually or increase similarity threshold.")
            results.append("")
            results.append("To override this safety check, set minSemanticSimilarity=0.0 (NOT RECOMMENDED)")
            results.append("BLOCKED")
        else:
            results.append("All variants passed semantic validation.")
            results.append("")
    except Exception as ex:
        results.append(f"WARNING: Could not perform semantic validation: {str(ex)}")
        results.append("Proceeding without semantic checks.")
        results.append("")

    return results


def process_wikilink_replacement(
    content: str,
    variants: Set[str],
    canonical_concept: str,
    removing_brackets: bool
) -> Tuple[str, int, List[str]]:
    """
    Process WikiLink replacement in content.

    Args:
        content: Markdown content
        variants: Set of concept variants to replace
        canonical_concept: Canonical concept to use
        removing_brackets: Whether to remove brackets entirely

    Returns:
        Tuple of (new_content, replacement_count, replacements_list)
    """
    new_content = content
    replacements = []
    total_replacements = 0

    matches = list(_WIKILINK_PATTERN.finditer(content))

    # Process matches in reverse order to preserve positions
    for match in reversed(matches):
        concept = match.group(1)
        normalized_concept = normalize_concept(concept)

        if normalized_concept in variants:
            old_link = match.group(0)
            new_link = concept if removing_brackets else f"[[{canonical_concept}]]"

            if old_link != new_link:
                start, end = match.span()
                new_content = new_content[:start] + new_link + new_content[end:]
                replacements.insert(0, f"  {old_link} → {new_link}")
                total_replacements += 1

    return (new_content, total_replacements, replacements)


def process_wikilink_creation(
    content: str,
    variants: Set[str],
    canonical_concept: str
) -> Tuple[str, int, List[str]]:
    """
    Create WikiLinks from plain text mentions.

    Args:
        content: Markdown content
        variants: Set of concept variants to find
        canonical_concept: Canonical concept to use

    Returns:
        Tuple of (new_content, replacement_count, replacements_list)
    """
    new_content = content
    replacements = []
    total_replacements = 0

    # Remove code blocks to avoid replacing in code
    # This is a simplified version; the C# uses Markdig AST
    code_block_pattern = re.compile(r"```.*?```", re.DOTALL)
    inline_code_pattern = re.compile(r"`[^`]+`")

    # Find code block positions to skip
    code_blocks = [(m.start(), m.end()) for m in code_block_pattern.finditer(content)]
    inline_codes = [(m.start(), m.end()) for m in inline_code_pattern.finditer(content)]

    def is_in_code(pos: int) -> bool:
        """Check if position is within code block or inline code."""
        for start, end in code_blocks + inline_codes:
            if start <= pos < end:
                return True
        return False

    # Process each variant
    for variant in sorted(variants, key=len, reverse=True):
        # Word boundary pattern
        pattern = re.compile(r'\b' + re.escape(variant) + r'\b', re.IGNORECASE)

        matches = list(pattern.finditer(content))

        # Process matches in reverse order
        for match in reversed(matches):
            start, end = match.span()

            # Skip if in code block
            if is_in_code(start):
                continue

            # Check if already a WikiLink (surrounded by [[]])
            if start >= 2 and content[start-2:start] == "[[":
                continue
            if end + 2 <= len(content) and content[end:end+2] == "]]":
                continue

            old_text = match.group(0)
            new_text = f"[[{canonical_concept}]]"

            new_content = new_content[:start] + new_text + new_content[end:]
            replacements.insert(0, f"  {old_text} → {new_text}")
            total_replacements += 1

    return (new_content, total_replacements, replacements)


def process_concept_replacements(
    md_files: List[str],
    variants: Set[str],
    canonical_concept: str,
    create_wikilinks: bool,
    removing_brackets: bool,
    dry_run: bool,
    memory_path: str
) -> Tuple[int, int, List[str]]:
    """
    Process concept replacements across all files.

    Args:
        md_files: List of markdown file paths
        variants: Set of concept variants to replace
        canonical_concept: Canonical concept to use
        create_wikilinks: Whether creating WikiLinks from plain text
        removing_brackets: Whether removing brackets entirely
        dry_run: Whether to simulate without writing
        memory_path: Base memory path for relative paths

    Returns:
        Tuple of (files_modified, total_replacements, results_list)
    """
    total_replacements = 0
    files_modified = 0
    results = []

    for file_path in md_files:
        try:
            # Read file
            frontmatter, content, _ = read_markdown(file_path)

            # Process replacements
            if create_wikilinks:
                new_content, replacement_count, file_replacements = process_wikilink_creation(
                    content, variants, canonical_concept
                )
            else:
                new_content, replacement_count, file_replacements = process_wikilink_replacement(
                    content, variants, canonical_concept, removing_brackets
                )

            if file_replacements:
                relative_path = os.path.relpath(file_path, memory_path)
                files_modified += 1
                total_replacements += replacement_count

                if not dry_run:
                    write_markdown(file_path, frontmatter, new_content)
                    results.append(f"✓ Modified: {relative_path}")
                else:
                    results.append(f"Would modify: {relative_path}")

                results.extend(file_replacements)

        except Exception as ex:
            results.append(f"ERROR processing {file_path}: {str(ex)}")

    return (files_modified, total_replacements, results)


def format_scan_initialization(
    results: List[str],
    file_count: int,
    variants: Set[str],
    canonical_concept: str,
    create_wikilinks: bool,
    removing_brackets: bool,
    normalized_canonical: str
) -> None:
    """Format and append scan initialization messages."""
    results.append(f"Scanning {file_count} markdown files...")

    if create_wikilinks:
        results.append(f"Looking for plain text: {', '.join(sorted(variants))}")
        results.append(f"Will CREATE WikiLinks: text → [[{canonical_concept}]]")
    else:
        results.append(f"Looking for variants: {', '.join(sorted(variants))}")
        if removing_brackets:
            results.append("Will REMOVE WikiLink brackets entirely (converting to plain text)")
        else:
            results.append(f"Will replace with: [[{canonical_concept}]] (normalized: {normalized_canonical})")

    results.append("")


def format_summary(
    results: List[str],
    file_count: int,
    files_modified: int,
    total_replacements: int,
    dry_run: bool
) -> None:
    """Format and append summary messages."""
    results.append("")
    results.append("=== SUMMARY ===")
    results.append(f"Files scanned: {file_count}")
    results.append(f"Files {'to modify' if dry_run else 'modified'}: {files_modified}")
    results.append(f"Total replacements: {total_replacements}")

    if dry_run and files_modified > 0:
        results.append("")
        results.append("This was a DRY RUN. To apply changes, run with dryRun=false")
        results.append("")
        results.append("After applying changes, run 'sync' to rebuild the graph with clean concepts.")
    elif not dry_run and files_modified > 0:
        results.append("")
        results.append("✓ Changes applied successfully!")
        results.append("Run 'sync' to rebuild the graph with the cleaned concepts.")


def repair_concepts(
    concepts_to_replace: str,
    canonical_concept: str,
    folder: Optional[str] = None,
    dry_run: bool = True,
    create_wikilinks: bool = False,
    min_semantic_similarity: float = 0.7,
    learn: bool = False
) -> str:
    """
    Repair graph corruption by replacing concept variants with canonical form.

    ⚠️ DANGER: This tool can PERMANENTLY DAMAGE the knowledge graph if used incorrectly!

    Repairs graph corruption by replacing concept variants with canonical form in source
    markdown files. Fixes the source of truth rather than just the database.

    SEMANTIC VALIDATION: Automatically blocks unsafe consolidations by checking semantic
    similarity between concepts. Only proceeds if ALL variants have similarity >=
    minSemanticSimilarity threshold (default 0.7).

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

    Test with dryRun=true first to see what would be changed.

    Args:
        concepts_to_replace: Comma/semicolon/pipe-separated list of variants to replace
        canonical_concept: The canonical concept to use (must be semantically equivalent)
        folder: Optional folder to process (default: all memory)
        dry_run: If True, shows what would change without modifying files (default: True)
        create_wikilinks: If True, creates WikiLinks from plain text instead of replacing existing
        min_semantic_similarity: Minimum similarity threshold (0.0-1.0, default: 0.7)
        learn: If True, returns help documentation

    Returns:
        Detailed report of changes made or to be made
    """
    if learn:
        return """# RepairConcepts

⚠️ DANGER: This tool can PERMANENTLY DAMAGE the knowledge graph if used incorrectly!

Repairs graph corruption by replacing concept variants with canonical form in source markdown files.

## Parameters
- concepts_to_replace (required): Array of concept variants to replace
- canonical_concept (required): The canonical concept to use
- folder (optional): Folder to process (default: all memory)
- dry_run (default: true): Always start with true! Shows what would be changed
- create_wikilinks (default: false): Create WikiLinks from plain text
- min_semantic_similarity (default: 0.7): Minimum similarity threshold (0.0-1.0)

## Safety Features
- Semantic validation blocks unsafe consolidations
- Dry run mode shows changes before applying
- Per-file error handling prevents partial corruption

## Examples
Safe consolidation:
  repair_concepts("tools,MCP tools", "tool", dry_run=True)

## Integration
- Use AnalyzeConceptCorruption FIRST to understand changes
- Run Sync after successful repair to rebuild graph
"""

    # Validate inputs
    variants, normalized_canonical, removing_brackets, error = validate_input(
        concepts_to_replace, canonical_concept, create_wikilinks
    )

    if error:
        return error

    # Determine search path
    memory_path = Config.memory_path
    search_path = memory_path if not folder else os.path.join(memory_path, folder)

    if not os.path.exists(search_path):
        return f"ERROR: Directory not found: {search_path}"

    # Semantic validation
    results = []
    if min_semantic_similarity > 0.0 and not removing_brackets and not create_wikilinks:
        validation_results = validate_semantic_similarity(
            variants, normalized_canonical, min_semantic_similarity
        )
        results.extend(validation_results)

        if validation_results and validation_results[-1] == "BLOCKED":
            results.pop()  # Remove BLOCKED marker
            return "\n".join(results)

    # Find markdown files
    md_files = []
    for root, dirs, files in os.walk(search_path):
        # Skip .git directories
        dirs[:] = [d for d in dirs if d != '.git']

        for file in files:
            if file.endswith('.md'):
                md_files.append(os.path.join(root, file))

    md_files.sort()

    # Format initialization
    format_scan_initialization(
        results, len(md_files), variants, canonical_concept,
        create_wikilinks, removing_brackets, normalized_canonical
    )

    # Process replacements
    files_modified, total_replacements, processing_results = process_concept_replacements(
        md_files, variants, canonical_concept, create_wikilinks,
        removing_brackets, dry_run, memory_path
    )
    results.extend(processing_results)

    # Format summary
    format_summary(results, len(md_files), files_modified, total_replacements, dry_run)

    return "\n".join(results)


def analyze_concepts(memory_path: str, concept_family: str) -> Dict[str, int]:
    """
    Analyze all concepts in memory that match a concept family.

    Args:
        memory_path: Path to memory directory
        concept_family: Concept family to search for (e.g., 'tool', 'agent')

    Returns:
        Dictionary of concept -> occurrence count
    """
    concept_counts: Dict[str, int] = {}

    # Find all markdown files
    md_files = []
    for root, dirs, files in os.walk(memory_path):
        # Skip .git directories
        dirs[:] = [d for d in dirs if d != '.git']

        for file in files:
            if file.endswith('.md'):
                md_files.append(os.path.join(root, file))

    # Extract concepts from each file
    family_lower = concept_family.lower()

    for file_path in md_files:
        try:
            with open(file_path, 'r', encoding='utf-8') as f:
                content = f.read()

            matches = _WIKILINK_PATTERN.findall(content)

            for concept in matches:
                normalized = normalize_concept(concept)

                if family_lower in normalized:
                    if concept not in concept_counts:
                        concept_counts[concept] = 0
                    concept_counts[concept] += 1

        except Exception:
            # Silently skip files that can't be read
            pass

    return concept_counts


def classify_concept_pattern(concept: str) -> str:
    """
    Classify a concept into a pattern category.

    Args:
        concept: Concept to classify

    Returns:
        Pattern category name
    """
    if concept.startswith('/') or '.' in concept:
        return "File Paths"

    if concept.lower().endswith('s') and not concept.lower().endswith('ss'):
        return "Singular/Plural"

    if '-' in concept:
        return "Compound (with -)"

    if any(suffix in concept.lower() for suffix in ['system', 'framework', 'architecture', 'tool']):
        return "With Suffix"

    return "Other"


def classify_concept_patterns(
    concept_counts: Dict[str, int],
    max_results: int
) -> Dict[str, List[str]]:
    """
    Classify concepts into pattern categories.

    Args:
        concept_counts: Dictionary of concept -> count
        max_results: Maximum number of results to classify

    Returns:
        Dictionary of pattern -> list of concepts
    """
    patterns: Dict[str, List[str]] = {
        "Singular/Plural": [],
        "Compound (with -)": [],
        "With Suffix": [],
        "File Paths": [],
        "Other": []
    }

    # Sort by count descending, take top max_results
    sorted_concepts = sorted(concept_counts.items(), key=lambda x: x[1], reverse=True)

    for concept, count in sorted_concepts[:max_results]:
        pattern_key = classify_concept_pattern(concept)
        patterns[pattern_key].append(f"{concept} ({count}x)")

    return patterns


def format_pattern_results(results: List[str], patterns: Dict[str, List[str]]) -> None:
    """Format and append pattern results."""
    for pattern_name, concepts in patterns.items():
        if concepts:
            results.append(f"{pattern_name}:")
            for concept in concepts:
                results.append(f"  • {concept}")
            results.append("")


def format_suggested_repairs(
    results: List[str],
    concept_counts: Dict[str, int],
    concept_family: str
) -> None:
    """Format and append suggested repairs."""
    results.append("=== SUGGESTED REPAIRS ===")

    # Find plural forms
    plurals = [
        c for c in concept_counts.keys()
        if c.lower().endswith('s') and not c.lower().endswith('ss')
    ]

    if plurals:
        singular = concept_family.rstrip('s')
        results.append("Fix plural forms:")
        plural_list = ','.join(plurals[:10])
        results.append(f"  RepairConcepts conceptsToReplace='{plural_list}' canonicalConcept='{singular}'")
        results.append("")

    # Find compound forms
    family_lower = concept_family.lower()
    compounds = [
        c for c in concept_counts.keys()
        if f"-{family_lower}" in c.lower() or f"{family_lower}-" in c.lower()
    ]

    if compounds:
        results.append("Fix compound forms:")
        compound_list = ','.join(compounds[:10])
        results.append(f"  RepairConcepts conceptsToReplace='{compound_list}' canonicalConcept='{concept_family}'")


def analyze_concept_corruption(
    concept_family: str,
    max_results: int = 50,
    learn: bool = False
) -> str:
    """
    Analyze concept corruption patterns to identify what needs repair.

    ⚠️ MUST USE BEFORE RepairConcepts! Analyzes concept corruption patterns to identify
    what needs repair. Shows concept families and their variants to help plan SAFE repair
    operations.

    CRITICAL: This tool helps you UNDERSTAND concepts before changing them:
    - Identifies TRUE duplicates (plurals, case variations) that are safe to merge
    - Shows class names and entity types that MUST NOT be merged with generic concepts
    - Reveals the semantic structure of your knowledge graph

    INTERPRETATION GUIDE:
    - File Paths (e.g., GraphTools.cs): These are CLASS NAMES - DO NOT merge with [[tool]]
    - Compound forms (e.g., coding-agent): These are SPECIFIC TYPES - DO NOT merge with [[agent]]
    - Plurals (e.g., tools): Usually safe to merge with singular form
    - Case variants (e.g., MCP tools vs MCP Tools): Safe to standardize

    Always analyze BEFORE repair to avoid destroying semantic meaning!

    Args:
        concept_family: Concept family to analyze (e.g., 'tool', 'agent', 'test')
        max_results: Maximum number of variants to show (default: 50)
        learn: If True, returns help documentation

    Returns:
        Analysis report showing concept variants and suggested repairs
    """
    if learn:
        return """# AnalyzeConceptCorruption

⚠️ MUST USE BEFORE RepairConcepts!

Analyzes concept corruption patterns to identify what needs repair.

## Parameters
- concept_family (required): Concept family to analyze (e.g., 'tool', 'agent', 'test')
- max_results (default: 50): Maximum variants to show

## Pattern Categories
- Singular/Plural: Usually safe to consolidate
- Compound (with -): Often specific types, review carefully
- With Suffix: May be specific implementations
- File Paths: CLASS NAMES - DO NOT merge with generic concepts
- Other: Miscellaneous patterns

## Interpretation Guide
- File paths/class names: DO NOT merge with generic concepts
- Compound forms: May be specific types - review semantic meaning
- Plurals: Usually safe to merge with singular
- Case variants: Safe to standardize

## Example
analyze_concept_corruption("tool", max_results=50)

## Integration
- ALWAYS use this before RepairConcepts
- Use FindSimilarConcepts to check semantic similarity
- Review patterns carefully before consolidating
"""

    memory_path = Config.memory_path
    concept_counts = analyze_concepts(memory_path, concept_family)

    results = []
    results.append(f"=== Concept Family Analysis: '{concept_family}' ===")
    results.append(f"Found {len(concept_counts)} unique variants")
    results.append("")

    patterns = classify_concept_patterns(concept_counts, max_results)
    format_pattern_results(results, patterns)

    format_suggested_repairs(results, concept_counts, concept_family)

    return "\n".join(results)
