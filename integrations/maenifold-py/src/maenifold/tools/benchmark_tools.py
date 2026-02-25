"""
Performance benchmark tools for maenifold.

This module provides comprehensive benchmarking for:
- GRPH-009: CTE vs N+1 query patterns
- Search performance (Hybrid, Semantic, FullText)
- Sync timing and throughput
- Complex traversal bottlenecks (5-8s scenarios)

Validates performance claims and identifies optimization opportunities.
"""

import os
import sqlite3
import time
from collections import deque
from typing import List, Tuple

from maenifold.config import Config
from maenifold.tools.concept_sync import sync
from maenifold.tools.graph_database import initialize_database
from maenifold.tools.graph_tools import build_context
from maenifold.tools.search_tools import search_memories
from maenifold.utils.sqlite_extensions import open_with_wal, query, query_single


def run_full_benchmark(
    iterations: int = 5,
    max_test_files: int = 1000,
    include_deep_traversal: bool = True,
    learn: bool = False
) -> str:
    """
    Benchmarks all maenifold performance claims.

    Validates:
    - GRPH-009: CTE vs N+1 query patterns (34% faster claim)
    - Search performance (Hybrid 33ms, Semantic 116ms, FullText 47ms)
    - Sync timing (27s for 2,500 files)
    - Complex traversal bottlenecks (5-8s scenarios)

    Args:
        iterations: Number of test iterations per benchmark (default 5)
        max_test_files: Maximum test files to use for benchmarks (default 1000)
        include_deep_traversal: Include expensive deep traversal tests (default True)
        learn: Return help documentation instead of executing (default False)

    Returns:
        Comprehensive benchmark report with timing results and claim verification

    Raises:
        ValueError: If iterations <= 0
    """
    if learn:
        return _get_help_text()

    # ISSUE-006: Validate iterations parameter
    if iterations <= 0:
        raise ValueError("iterations must be > 0")

    results = []
    results.append("MA CORE PERFORMANCE BENCHMARK SUITE")
    results.append("=" * 60)
    results.append(f"Iterations: {iterations}, Max Test Files: {max_test_files}")
    results.append(f"Started: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    results.append("")

    try:
        # Initialize database
        initialize_database()

        # Run benchmark suites
        results.append(_benchmark_graph_traversal(iterations))
        results.append("")

        results.append(_benchmark_search_performance(iterations, max_test_files))
        results.append("")

        results.append(_benchmark_sync_performance(iterations, max_test_files))
        results.append("")

        if include_deep_traversal:
            results.append(_benchmark_complex_traversal(iterations))
            results.append("")

        results.append(_generate_system_health_report())

    except Exception as ex:
        results.append("BENCHMARK FAILED:")
        results.append(str(ex))

    results.append(f"Completed: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    return "\n".join(results)


def _benchmark_graph_traversal(iterations: int) -> str:
    """
    Benchmark GRPH-009: CTE vs N+1 pattern performance.

    Tests multi-hop graph traversal using:
    - Optimized CTE-based implementation (current)
    - N+1 pattern implementation (baseline comparison)

    Args:
        iterations: Number of iterations per test concept

    Returns:
        Formatted benchmark results with claim verification
    """
    results = []
    results.append("GRPH-009: CTE vs N+1 Pattern Benchmark")
    results.append("Claim: 34% faster than CTE")
    results.append("")

    conn = sqlite3.connect(Config.database_path)
    open_with_wal(conn)

    # Find test concepts with good connectivity
    test_concepts = query(
        conn,
        """
        SELECT concept_name, relation_count
        FROM (
            SELECT concept_name,
                   (SELECT COUNT(*) FROM concept_graph
                    WHERE concept_a = c.concept_name OR concept_b = c.concept_name) as relation_count
            FROM concepts c
        )
        WHERE relation_count > 5
        ORDER BY relation_count DESC
        LIMIT 10
        """
    )

    if not test_concepts:
        conn.close()
        return "No test concepts with sufficient connectivity found. Run sync first."

    cte_timings = []
    n1_timings = []

    for concept_name, relation_count in test_concepts[:3]:
        results.append(f"Testing concept: {concept_name} (relations: {relation_count})")

        # Benchmark CTE-based implementation
        for i in range(iterations):
            start = time.time()
            # Use build_context which uses optimized CTE traversal
            cte_result = build_context(concept_name, depth=3, max_entities=50)
            elapsed_ms = (time.time() - start) * 1000
            cte_timings.append(elapsed_ms)

        # Benchmark N+1 pattern
        for i in range(iterations):
            start = time.time()
            n1_result = _benchmark_n1_pattern(conn, concept_name, depth=3, max_nodes=50)
            elapsed_ms = (time.time() - start) * 1000
            n1_timings.append(elapsed_ms)

    conn.close()

    # Calculate averages
    cte_avg = sum(cte_timings) / len(cte_timings)
    n1_avg = sum(n1_timings) / len(n1_timings)
    percent_diff = ((cte_avg - n1_avg) / cte_avg) * 100

    results.append("")
    results.append(f"Results ({iterations * len(test_concepts[:3])} iterations each):")
    results.append(f"CTE Average: {cte_avg:.1f}ms")
    results.append(f"N+1 Average: {n1_avg:.1f}ms")
    results.append(f"Performance difference: {percent_diff:.1f}% {'N+1 FASTER' if percent_diff > 0 else 'CTE FASTER'}")
    results.append(f"Claim verification: {'CONFIRMED' if abs(percent_diff) >= 34 else 'UNCONFIRMED'} (target: 34%)")

    return "\n".join(results)


def _benchmark_n1_pattern(
    conn: sqlite3.Connection,
    concept_name: str,
    depth: int,
    max_nodes: int
) -> str:
    """
    Simulate N+1 query pattern for comparison.

    This implements a naive breadth-first traversal that issues one query
    per node, similar to the O(N) anti-pattern we're optimizing against.

    Args:
        conn: SQLite connection
        concept_name: Starting concept
        depth: Maximum traversal depth
        max_nodes: Maximum nodes to visit

    Returns:
        Mermaid diagram string (for consistency with CTE version)
    """
    visited = set()
    relations = []
    queue = deque([(concept_name, 0)])
    visited.add(concept_name)

    while queue and len(relations) < max_nodes:
        current, current_depth = queue.popleft()

        if current_depth >= depth:
            continue

        # N+1 pattern: One query per node
        neighbors = query(
            conn,
            """
            SELECT
                CASE WHEN concept_a = ? THEN concept_b ELSE concept_a END as neighbor,
                co_occurrence_count as count
            FROM concept_graph
            WHERE concept_a = ? OR concept_b = ?
            ORDER BY co_occurrence_count DESC
            """,
            (current, current, current)
        )

        for neighbor, count in neighbors:
            relations.append((current, neighbor, count))

            if neighbor not in visited and len(relations) < max_nodes:
                visited.add(neighbor)
                queue.append((neighbor, current_depth + 1))

    # Generate Mermaid diagram (for consistency)
    mermaid_lines = ["graph TD"]
    for a, b, count in relations[:max_nodes]:
        node_a = a.replace(" ", "_").replace("-", "_")
        node_b = b.replace(" ", "_").replace("-", "_")
        mermaid_lines.append(f"    {node_a} -->|{count}| {node_b}")

    return "\n".join(mermaid_lines)


def _benchmark_search_performance(iterations: int, max_test_files: int) -> str:
    """
    Benchmark search performance across all modes.

    Tests:
    - Hybrid search (claim: 33ms)
    - Semantic search (claim: 116ms)
    - Full-text search (claim: 47ms)

    Args:
        iterations: Number of iterations per query
        max_test_files: Maximum files to test (unused, for API compatibility)

    Returns:
        Formatted benchmark results with claim verification
    """
    results = []
    results.append("Search Performance Benchmark")
    results.append("Claims: Hybrid 33ms, Semantic 116ms, Full-text 47ms")
    results.append("")

    # Count files in memory
    file_count = len([
        f for f in os.listdir(Config.memory_path)
        if f.endswith(".md")
    ])
    for root, _, files in os.walk(Config.memory_path):
        file_count += len([f for f in files if f.endswith(".md")])

    results.append(f"Test dataset: {file_count} files")

    if file_count < 100:
        return f"Insufficient test data ({file_count} files). Need at least 100 files for meaningful benchmarks."

    # Test queries
    test_queries = [
        "machine learning algorithms",
        "performance optimization",
        "graph database",
        "vector embeddings",
        "concept relationships"
    ]

    hybrid_timings = []
    semantic_timings = []
    fulltext_timings = []

    for query_text in test_queries:
        for i in range(iterations):
            # Benchmark Hybrid
            start = time.time()
            hybrid_result = search_memories(query_text, mode="Hybrid", page_size=10)
            elapsed_ms = (time.time() - start) * 1000
            hybrid_timings.append(elapsed_ms)

            # Benchmark Semantic
            start = time.time()
            semantic_result = search_memories(query_text, mode="Semantic", page_size=10)
            elapsed_ms = (time.time() - start) * 1000
            semantic_timings.append(elapsed_ms)

            # Benchmark FullText
            start = time.time()
            fulltext_result = search_memories(query_text, mode="FullText", page_size=10)
            elapsed_ms = (time.time() - start) * 1000
            fulltext_timings.append(elapsed_ms)

    hybrid_avg = sum(hybrid_timings) / len(hybrid_timings)
    semantic_avg = sum(semantic_timings) / len(semantic_timings)
    fulltext_avg = sum(fulltext_timings) / len(fulltext_timings)

    results.append("")
    results.append(f"Results ({iterations * len(test_queries)} iterations each):")
    results.append(f"Hybrid Average: {hybrid_avg:.1f}ms (claim: 33ms)")
    results.append(f"Semantic Average: {semantic_avg:.1f}ms (claim: 116ms)")
    results.append(f"Full-text Average: {fulltext_avg:.1f}ms (claim: 47ms)")

    return "\n".join(results)


def _benchmark_sync_performance(iterations: int, max_test_files: int) -> str:
    """
    Benchmark sync performance and throughput.

    Tests full sync operation and projects performance for large datasets.
    Claim: 27s for 2,500 files

    Args:
        iterations: Number of sync iterations (limited to 3 max)
        max_test_files: Maximum files to test (unused, for API compatibility)

    Returns:
        Formatted benchmark results with claim verification
    """
    results = []
    results.append("Sync Performance Benchmark")
    results.append("Claim: 27s for 2,500 files")
    results.append("")

    # Count files
    file_count = 0
    for root, _, files in os.walk(Config.memory_path):
        file_count += len([f for f in files if f.endswith(".md")])

    results.append(f"Test dataset: {file_count} files")

    sync_timings = []

    # Run up to 3 sync iterations (can be expensive)
    for i in range(min(iterations, 3)):
        start = time.time()
        sync_result = sync()
        elapsed_ms = (time.time() - start) * 1000
        sync_timings.append(elapsed_ms)

        results.append(f"Sync iteration {i + 1}: {elapsed_ms:.0f}ms")

    avg_sync_time = sum(sync_timings) / len(sync_timings)
    projected_time_for_2500 = (avg_sync_time / file_count) * 2500 if file_count > 0 else 0

    results.append("")
    results.append("Results:")
    results.append(f"Average sync time: {avg_sync_time:.0f}ms for {file_count} files")
    results.append(f"Projected time for 2,500 files: {projected_time_for_2500:.0f}ms ({projected_time_for_2500 / 1000:.1f}s)")
    results.append(f"Claim verification: {'REASONABLE' if projected_time_for_2500 / 1000 <= 35 else 'SLOWER THAN CLAIMED'} (target: 27s)")

    return "\n".join(results)


def _benchmark_complex_traversal(iterations: int) -> str:
    """
    Benchmark complex traversal bottlenecks.

    Identifies 5-8s scenarios by testing deep traversal on highly
    connected hub concepts.

    Args:
        iterations: Number of iterations per hub concept

    Returns:
        Formatted benchmark results with bottleneck analysis
    """
    results = []
    results.append("Complex Traversal Bottleneck Analysis")
    results.append("Target: Identify 5-8s complex traversal scenarios")
    results.append("")

    conn = sqlite3.connect(Config.database_path)
    open_with_wal(conn)

    # Find hub concepts with high connectivity
    hub_concepts = query(
        conn,
        """
        SELECT concept_name, connection_count
        FROM (
            SELECT concept_name,
                   (SELECT COUNT(*) FROM concept_graph
                    WHERE concept_a = c.concept_name OR concept_b = c.concept_name) as connection_count
            FROM concepts c
        )
        ORDER BY connection_count DESC
        LIMIT 5
        """
    )

    conn.close()

    if not hub_concepts:
        return "No concepts found for traversal testing. Run sync first."

    complex_timings = []

    for concept_name, connections in hub_concepts:
        results.append(f"Testing hub concept: {concept_name} ({connections} connections)")

        # Deep traversal test (depth=5, maxNodes=200)
        for i in range(iterations):
            start = time.time()
            result = build_context(concept_name, depth=5, max_entities=200)
            elapsed_ms = (time.time() - start) * 1000
            complex_timings.append(elapsed_ms)

            if elapsed_ms > 1000:
                results.append(f"  Slow traversal detected: {elapsed_ms:.0f}ms")

    avg_complex_time = sum(complex_timings) / len(complex_timings)
    max_time = max(complex_timings)
    slow_count = sum(1 for t in complex_timings if t >= 5000)

    results.append("")
    results.append("Complex Traversal Results:")
    results.append(f"Average time: {avg_complex_time:.1f}ms")
    results.append(f"Maximum time: {max_time:.1f}ms")
    results.append(f"Queries >=5s: {slow_count}/{len(complex_timings)}")
    results.append(f"Bottleneck confirmed: {'YES' if slow_count > 0 else 'NO'} (target: 5-8s scenarios)")

    return "\n".join(results)


def _generate_system_health_report() -> str:
    """
    Generate system health report with database metrics.

    Returns:
        Formatted health report with concept counts, relations, and sizes
    """
    results = []
    results.append("System Health Report")
    results.append("")

    try:
        conn = sqlite3.connect(Config.database_path)
        open_with_wal(conn)

        concept_count = query_single(conn, "SELECT COUNT(*) FROM concepts") or 0
        relation_count = query_single(conn, "SELECT COUNT(*) FROM concept_graph") or 0
        file_count = query_single(conn, "SELECT COUNT(*) FROM file_content") or 0

        results.append("Database health:")
        results.append(f"  Concepts: {concept_count:,}")
        results.append(f"  Relations: {relation_count:,}")
        results.append(f"  Files indexed: {file_count:,}")

        # Try to get vector stats
        try:
            from maenifold.utils.sqlite_extensions import load_vector_extension
            load_vector_extension(conn)

            vec_concepts = query_single(
                conn,
                "SELECT COUNT(*) FROM vec_concepts WHERE embedding IS NOT NULL"
            ) or 0
            vec_files = query_single(
                conn,
                "SELECT COUNT(*) FROM vec_memory_files WHERE embedding IS NOT NULL"
            ) or 0

            results.append(f"  Vector embeddings: {vec_concepts:,} concepts, {vec_files:,} files")
        except Exception:
            results.append("  Vector embeddings: sqlite-vec unavailable")

        # Database size
        if os.path.exists(Config.database_path):
            db_size_mb = os.path.getsize(Config.database_path) / (1024.0 * 1024.0)
            results.append(f"  Database size: {db_size_mb:.1f} MB")

        conn.close()

    except Exception as ex:
        results.append(f"Health check failed: {ex}")

    return "\n".join(results)


def _get_help_text() -> str:
    """Return help documentation for run_full_benchmark."""
    return """RunFullBenchmark - Comprehensive performance validation

Benchmarks all maenifold performance claims:

1. GRPH-009: CTE vs N+1 Pattern
   - Tests optimized CTE-based graph traversal
   - Compares against naive N+1 query pattern
   - Validates 34% performance improvement claim

2. Search Performance
   - Hybrid search (claim: 33ms avg)
   - Semantic search (claim: 116ms avg)
   - Full-text search (claim: 47ms avg)
   - Tests on actual knowledge base content

3. Sync Performance
   - Full sync timing with real files
   - Projects performance for 2,500 files
   - Validates 27s claim for large datasets

4. Complex Traversal Bottlenecks
   - Deep graph traversal (depth=5, 200 nodes)
   - Identifies 5-8s scenarios on hub concepts
   - Stress tests on highly connected nodes

Parameters:
- iterations: Test iterations per benchmark (default 5)
- max_test_files: Max files for benchmarks (default 1000)
- include_deep_traversal: Include expensive traversal tests (default True)
- learn: Return this help instead of executing (default False)

Returns:
Multi-section report with:
- Timing results for each benchmark
- Claim verification (CONFIRMED/UNCONFIRMED)
- System health metrics
- Database statistics

Notes:
- Requires synced database with indexed content
- Deep traversal can be expensive (skip with include_deep_traversal=False)
- Run with at least 100 files for meaningful results
- Sync benchmarks limited to 3 iterations (expensive operation)
"""
