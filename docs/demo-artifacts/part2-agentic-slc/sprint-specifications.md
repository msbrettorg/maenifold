# Sprint Specifications
## Date: 2025-01-21
## Session: workflow-1758474734596

## Functional Requirements

### FR-1: SRCH-004 - Add minScore Parameter to Search
**Description**: Implement minScore threshold parameter to filter search results by similarity score
**Location**: src/Tools/MemorySearchTools.cs
**Changes Required**:
1. Add minScore parameter to SearchMemories method signature (line 45)
   - Type: double
   - Default: 0.0 (backward compatible)
   - Description: "Minimum similarity score threshold (0.0-1.0)"
2. Propagate minScore to all three search methods (lines 55, 57, 60)
3. Implement filtering after score calculation:
   - Hybrid: Filter by fusedScore after RRF fusion
   - Semantic: Filter by raw similarity score
   - Full-text: Normalize scores to 0-1 range first, then filter
**Acceptance Criteria**:
- minScore=0.0 returns all results (default behavior)
- minScore=1.0 returns only perfect matches
- All three search modes respect the parameter
- No breaking changes to existing API

### FR-2: MEM-009 - Verify/Fix Nested Directory Path Support
**Description**: Ensure nested directory paths are created correctly in WriteMemory
**Location**: src/Tools/MemoryTools.Write.cs
**Investigation Required**:
1. Test current behavior with path like "decisions/api/v2"
2. Verify Directory.CreateDirectory() is working as expected
3. If issue confirmed, add path validation (line 32):
   - Reject path traversal attempts ("..")
   - Reject absolute paths
   - Normalize path separators for cross-platform
**Acceptance Criteria**:
- Nested paths create proper directory hierarchy
- Path traversal protection implemented
- Cross-platform path compatibility ensured
- Existing single-level paths continue to work

### FR-3: GRPH-009 - Fix N+1 Query Problem in Graph Traversal
**Description**: Replace inefficient nested loop queries with recursive CTE approach
**Location**: src/Tools/GraphTools.cs (lines 89-101)
**Changes Required**:
1. Replace foreach loop with single recursive CTE query
2. Use same pattern as Visualize tool (GraphAnalyzer.cs:80-108)
3. Batch all second-hop queries into one SQL execution
4. Add LIMIT to prevent unbounded result sets
**Acceptance Criteria**:
- Single database query for all traversal levels
- Performance improvement from O(n²) to O(n)
- Result set limits to prevent memory issues
- Same output format as current implementation

## Non-Functional Requirements

### NFR-1: Performance Constraints
- Search operations must maintain <150ms response time
- Graph traversal must complete in <200ms for typical graphs
- No performance regression in other operations
- Memory usage must not exceed current baseline

### NFR-2: Security Requirements
- Path traversal protection in WriteMemory
- SQL injection prevention in all queries
- No exposure of system paths in error messages

### NFR-3: Compatibility Constraints
- All changes must be backward compatible
- Default parameter values preserve existing behavior
- CLI and MCP interfaces must remain identical
- No breaking changes to API contracts

## Implementation Constraints

### Technology Stack
- Language: C# with .NET 9.0
- Database: SQLite with vector extension
- Testing: MSTest framework
- Build: TreatWarningsAsErrors enabled

### Dependencies
- No new NuGet packages required
- Use existing ONNX runtime for embeddings
- Maintain current SQLite version

### File Modifications
1. **src/Tools/MemorySearchTools.cs**
   - Lines 39-45: Method signature
   - Lines 55-60: Parameter propagation
   - Lines 77, 110, 148: Filtering logic

2. **src/Tools/MemoryTools.Write.cs**
   - Line 32: Path validation (if issue confirmed)

3. **src/Tools/GraphTools.cs**
   - Lines 89-101: Replace with CTE query

4. **tests/Maenifold.Tests/**
   - New test files for regression tests
   - Minimum 3 tests per fix

## Test Requirements

### Unit Tests
- SRCH-004: Test minScore filtering for all search modes
- MEM-009: Test nested path creation and validation
- GRPH-009: Test traversal performance with large graphs

### Integration Tests
- CLI mode: Verify all fixes work via command line
- MCP mode: Verify identical behavior in server mode
- Cross-feature: Ensure fixes don't interact negatively

## Risk Assessment

### Low Risk
- SRCH-004: Simple parameter addition with default value

### Medium Risk
- MEM-009: May need investigation before implementation

### High Risk
- GRPH-009: Algorithm change could affect output format

## Success Metrics
- All regression tests passing
- No performance degradation
- 100% backward compatibility
- Zero new warnings introduced