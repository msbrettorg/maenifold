# RunFullBenchmark

Comprehensive [[performance]] [[benchmark]] suite that validates all Maenifold performance claims including [[graph]] traversal ([[GRPH-009]] CTE vs N+1), [[search]] performance, [[sync]] timing, and complex traversal bottlenecks. Provides empirical validation of system performance characteristics under real workloads.

## When to Use This Tool

- **Performance Validation**: Verify Maenifold meets documented performance targets
- **System Health Checks**: Assess overall system performance before/after changes
- **Regression Detection**: Compare performance across versions or configurations
- **Capacity Planning**: Understand performance at different dataset scales
- **Optimization Verification**: Confirm performance improvements from code changes
- **Production Readiness**: Validate performance before deployment
- **Debugging Performance Issues**: Identify which subsystems are slow

## Key Features

- **Comprehensive Coverage**: Tests [[graph-traversal]], [[search]], [[sync]], and complex operations
- **Real Workloads**: Uses actual memory files and knowledge graph data
- **Statistical Rigor**: Multiple iterations with averaged results
- **Claim Validation**: Compares results against documented performance targets
- **System Health Report**: Database size, concept counts, memory statistics
- **Configurable**: Control iterations, test file limits, and expensive tests
- **Production-Safe**: Read-only operations, doesn't modify system state

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| iterations | int | No | Number of test iterations per benchmark (default: 5, more = more accurate) | 10 |
| maxTestFiles | int | No | Maximum test files to use for benchmarks (default: 1000, limits scope) | 500 |
| includeDeepTraversal | bool | No | Include expensive deep traversal tests (default: true, may timeout) | false |

## Usage Examples

### Standard Benchmark Run
```json
{
  "iterations": 5
}
```
Runs complete benchmark suite with 5 iterations per test - balances accuracy and time (~2-5 minutes).

### Quick Health Check
```json
{
  "iterations": 3,
  "includeDeepTraversal": false
}
```
Faster benchmark skipping expensive deep traversal tests - useful for quick validation (~1-2 minutes).

### High-Accuracy Benchmark
```json
{
  "iterations": 10,
  "maxTestFiles": 2000
}
```
More iterations and larger dataset for statistically robust results (~5-10 minutes).

### Small Dataset Testing
```json
{
  "iterations": 5,
  "maxTestFiles": 500
}
```
Limited test file scope for smaller knowledge bases or faster execution.

## Benchmark Suite Components

### 1. Graph Traversal Benchmark (GRPH-009)
**Tests**: CTE (Common Table Expression) vs N+1 query patterns
**Claims**: CTE recursive queries should outperform naive N+1 approaches
**Measures**: Query execution time for concept relationship traversal
**Validates**: [[graph-database]] optimization effectiveness

### 2. Search Performance Benchmark
**Tests**: Three search modes - Hybrid, Semantic, Full-text
**Claims**:
- Hybrid: ~33ms average
- Semantic: ~116ms average
- Full-text: ~47ms average

**Test Queries**:
- "machine learning algorithms"
- "performance optimization"
- "graph database"
- "vector embeddings"
- "concept relationships"

**Validates**: [[vector-search]] and [[full-text-search]] performance

### 3. Sync Performance Benchmark
**Tests**: Knowledge graph synchronization speed
**Claims**: ~27 seconds for 2,500 files
**Measures**: Time to scan files, extract [[WikiLinks]], generate [[embeddings]], update database
**Validates**: [[sync]] operation scalability

### 4. Complex Traversal Benchmark (Optional)
**Tests**: Deep [[graph]] traversal with multiple hops
**Claims**: Handles deep concept relationships efficiently
**Measures**: Multi-level BuildContext operations
**Validates**: Performance under complex query patterns
**Note**: ⚠️ Can timeout with large graphs - use includeDeepTraversal=false to skip

## Output Structure

### Header
```
MAENIFOLD PERFORMANCE BENCHMARK SUITE
=====================================
Iterations: 5, Max Test Files: 1000
Started: 2024-10-24 18:45:00
```

### Search Performance Results
```
Search Performance Benchmark
Claims: Hybrid 33ms, Semantic 116ms, Full-text 47ms

Test dataset: 2,458 files
Results (25 iterations each):
Hybrid Average: 34.2ms (claim: 33ms) ✓
Semantic Average: 118.7ms (claim: 116ms) ✓
Full-text Average: 45.3ms (claim: 47ms) ✓
```

### Sync Performance Results
```
Sync Performance Benchmark
Claim: 27s for 2,500 files

Test dataset: 2,458 files
Results (3 iterations):
Sync Average: 28.3s (claim: 27s) ✓
Files per second: 86.8
```

### Graph Traversal Results
```
Graph Traversal Benchmark (GRPH-009)
CTE vs N+1 Query Performance

CTE Average: 12.4ms
N+1 Average: 247.8ms
Performance Improvement: 19.9x faster ✓
```

### System Health Report
```
System Health Report
Database Size: 145.2 MB
Concept Count: 12,847
Memory Files: 2,458
Total WikiLinks: 47,392
Average Concepts per File: 19.3
```

### Footer
```
Completed: 2024-10-24 18:52:15
Total Duration: 7m 15s
```

## Performance Claim Validation

### ✓ Passing Criteria
Results within ±20% of claimed performance indicate system health:
- Hybrid search: 26-40ms (claim: 33ms)
- Semantic search: 93-140ms (claim: 116ms)
- Full-text search: 38-56ms (claim: 47ms)
- Sync: 22-32s for 2,500 files (claim: 27s)

### ⚠️ Warning Signs
Results 20-50% slower than claims suggest investigation:
- Database fragmentation (run VACUUM)
- Insufficient memory/CPU resources
- Disk I/O bottlenecks
- Large file size bloat

### 🚨 Failure Indicators
Results >50% slower indicate serious issues:
- Corrupted indices (rebuild database)
- System resource exhaustion
- Incorrect configuration
- Database locking contention

## Common Patterns

### Pre-Release Validation
```bash
# Before releasing new version
RunFullBenchmark iterations=10 includeDeepTraversal=true

# Verify all benchmarks pass
# Document any performance changes in CHANGELOG
```

### Post-Optimization Verification
```bash
# Before optimization
RunFullBenchmark iterations=5 > before.txt

# Apply optimization changes

# After optimization
RunFullBenchmark iterations=5 > after.txt

# Compare results
diff before.txt after.txt
```

### Continuous Integration
```bash
# In CI pipeline
RunFullBenchmark iterations=3 maxTestFiles=500 includeDeepTraversal=false

# Fast validation that catches major regressions
# Full benchmark in nightly builds
```

### Performance Debugging
```bash
# Identify slow subsystem
RunFullBenchmark iterations=10

# Review which benchmark fails or performs poorly
# Focus optimization efforts on that subsystem
```

## Related Tools

- **Sync**: Tested by sync performance benchmark - rebuild [[graph]] if needed
- **SearchMemories**: Tested by search performance benchmark - three search modes validated
- **BuildContext**: Tested by graph traversal benchmark - CTE optimization validated
- **MemoryStatus**: Compare with system health report for consistency checks
- **GetConfig**: Review configuration settings that affect performance

## Troubleshooting

### Error: "Insufficient test data (X files). Need at least 100 files for meaningful benchmarks"
**Cause**: Not enough memory files to produce statistically valid results
**Solution**:
- Add more test data (use WriteMemory to create sample memories)
- Or accept that benchmarks on small datasets aren't representative

### Benchmark Timeout (Deep Traversal)
**Cause**: Complex traversal tests can take >5 minutes with large graphs
**Solution**: Run with `includeDeepTraversal=false` to skip expensive tests

### Results Much Slower Than Claims
**Cause**: Database fragmentation, resource constraints, or system issues
**Solution**:
1. Run Sync to rebuild indices
2. Run SQLite VACUUM to defragment database
3. Check system resources (CPU, memory, disk I/O)
4. Review Config settings for performance tuning

### Results Highly Variable Between Runs
**Cause**: System resource contention or insufficient iterations
**Solution**:
- Increase iterations parameter (10-20 for stable results)
- Close other applications during benchmark
- Run on dedicated system without background load

### "BENCHMARK FAILED" Error
**Cause**: Exception during benchmark execution
**Solution**: Check error message details - may indicate database corruption, missing files, or configuration issues

## System Requirements for Valid Benchmarks

### Minimum Dataset Size
- **100+ files**: Required for search benchmarks
- **1,000+ files**: Recommended for realistic results
- **2,500+ files**: Matches performance claims benchmark conditions

### Hardware Considerations
- **RAM**: 4GB+ available (vector operations memory-intensive)
- **Storage**: SSD recommended (HDD will be much slower)
- **CPU**: Multi-core benefits sync and search operations

### Database Health
- Run Sync before benchmarking to ensure indices current
- Consider VACUUM if database heavily modified
- Verify no database locks or contention

## Interpreting Results

### When Results Match Claims
System is performing as designed - all optimizations working correctly.

### When Search Slower Than Expected
- Check vector extension loaded correctly
- Verify [[ONNX]] model loaded (first semantic search has ~400ms overhead)
- Review database indices with EXPLAIN QUERY PLAN

### When Sync Slower Than Expected
- Large files slow sync (Ma Protocol recommends <250 lines)
- Many [[WikiLinks]] per file increases processing
- Vector embedding generation is CPU-intensive

### When Graph Traversal Slow
- Deep traversal (depth >3) becomes expensive
- Highly connected concepts create large result sets
- CTE vs N+1 improvement validates optimization working

## Benchmark Best Practices

### DO
- Run multiple iterations (5-10) for stable results
- Use consistent dataset between runs for comparison
- Document system state (CPU load, other processes)
- Run Sync before benchmarking to ensure fresh indices

### DON'T
- Run benchmarks with active writes (skews results)
- Compare results across different dataset sizes
- Rely on single iteration (too much variance)
- Ignore system health report warnings

## Example Benchmark Session

### Step 1: Prepare System
```json
{
  "tool": "Sync"
}
```
Ensure database indices are current.

### Step 2: Run Benchmark
```json
{
  "iterations": 10,
  "maxTestFiles": 1000,
  "includeDeepTraversal": true
}
```

### Step 3: Review Results
Check each benchmark against claims:
- ✓ Search: All modes within expected range
- ✓ Sync: 28.3s for 2,458 files (expected ~27s for 2,500)
- ✓ Graph: CTE 19.9x faster than N+1
- ✓ System Health: No warnings

### Step 4: Document
Save benchmark output for historical comparison:
```bash
RunFullBenchmark > benchmark-v1.2.3.txt
```

## Ma Protocol Compliance

RunFullBenchmark follows Maenifold's Ma Protocol principles:
- **Real Testing**: Uses actual memory files and live database, no mocks
- **Transparent Metrics**: Reports exact timings and compares to documented claims
- **No Magic**: Standard Stopwatch timing with clear measurement points
- **Single Responsibility**: Measures performance, doesn't modify system
- **Statistical Rigor**: Multiple iterations with averages, not single-shot results
- **Production-Safe**: Read-only operations, safe to run on live systems

This tool embodies Ma Protocol's commitment to **measurement over assumption** - every performance claim is validated empirically with real data, ensuring the system performs as documented.
