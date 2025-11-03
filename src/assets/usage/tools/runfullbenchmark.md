# RunFullBenchmark

Validates Ma Core performance claims: GRPH-009 CTE vs N+1, search latency, sync timing, complex traversal bottlenecks.

## Parameters

- `iterations` (int, default: 5): Test iterations per benchmark
- `maxTestFiles` (int, default: 1000): Max files for benchmarks
- `includeDeepTraversal` (bool, default: true): Run expensive deep traversal tests (may timeout on large graphs)

## Returns

```text
MA CORE PERFORMANCE BENCHMARK SUITE
=====================================
Iterations: 5, Max Test Files: 1000
Started: 2025-11-03 10:30:00

GRPH-009: CTE vs N+1 Pattern Benchmark
Claim: 34% faster than N+1
CTE Average: 12.4ms, N+1 Average: 247.8ms
Performance difference: 19.9x faster ✓

Search Performance Benchmark
Claims: Hybrid 33ms, Semantic 116ms, Full-text 47ms
Test dataset: 2,458 files
Hybrid Average: 34.2ms (claim: 33ms) ✓
Semantic Average: 118.7ms (claim: 116ms) ✓
Full-text Average: 45.3ms (claim: 47ms) ✓

Sync Performance Benchmark
Claim: 27s for 2,500 files
Average sync time: 28.3s for 2,458 files
Projected time for 2,500 files: 28.9s (claim: 27s) ✓

Complex Traversal Bottleneck Analysis
Average time: 127.4ms
Maximum time: 892.1ms
Queries >=5s: 0/25

System Health Report
Database health:
  Concepts: 12,847
  Relations: 47,392
  Files indexed: 2,458
  Vector embeddings: 12,847 concepts, 2,458 files
  Database size: 145.2 MB

Completed: 2025-11-03 10:35:00
```

## Example

```json
{
  "iterations": 3,
  "maxTestFiles": 500,
  "includeDeepTraversal": false
}
```

## Constraints

- **Minimum dataset**: 100 files required for meaningful search benchmarks
- **Sync iterations**: Limited to 3 regardless of `iterations` parameter (sync is expensive)
- **Graph data required**: Run Sync before benchmarks for accurate graph metrics

## Benchmarks Executed

1. **GRPH-009 CTE vs N+1**: Tests concepts with >5 relations, depth 3, 50 nodes max. Validates 34% performance claim.
2. **Search Performance**: 5 test queries (ML, optimization, graph, vectors, concepts) against Hybrid/Semantic/Full-text modes. Claims: 33ms/116ms/47ms.
3. **Sync Performance**: Measures sync timing, projects to 2,500 files. Claim: 27s target.
4. **Complex Traversal** (optional): Hub concepts with most connections, depth 5, 200 nodes. Identifies 5-8s bottleneck scenarios.

## Interpreting Results

### ✓ Passing (within ±20% of claims)
- Hybrid: 26-40ms | Semantic: 93-140ms | Full-text: 38-56ms
- Sync: 22-32s for 2,500 files

### ⚠️ Warning (20-50% slower)
Database fragmentation, resource constraints, or disk I/O bottlenecks. Run VACUUM, check system resources.

### 🚨 Failure (>50% slower)
Corrupted indices (rebuild database), resource exhaustion, incorrect configuration, or locking contention.

## Common Use Cases

**Pre-release validation:**
```json
{"iterations": 10, "includeDeepTraversal": true}
```

**Quick health check (~1-2 min):**
```json
{"iterations": 3, "includeDeepTraversal": false}
```

**Post-optimization comparison:**
```bash
# Before: RunFullBenchmark iterations=5 > before.txt
# After: RunFullBenchmark iterations=5 > after.txt
# Compare: diff before.txt after.txt
```

## Integration

- **Sync**: Required before running - rebuilds graph indices
- **MemoryStatus**: Compare dataset size with benchmark report
- **SearchMemories/BuildContext/Visualize**: Tools being benchmarked - results affect UX

## Troubleshooting

**"Insufficient test data (X files)"**: Need 100+ files. Add test data or accept small dataset limitations.

**Benchmark timeout**: Complex traversal can exceed 5 min on large graphs. Use `includeDeepTraversal=false`.

**Results highly variable**: Increase iterations (10-20) or close other applications during benchmark.

**Results much slower than claims**: Run Sync, VACUUM database, check system resources (4GB+ RAM, SSD recommended).
