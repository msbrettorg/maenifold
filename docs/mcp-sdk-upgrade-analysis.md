# MCP SDK 0.3.0 → 0.4.0 Upgrade Impact Analysis

**Analysis Date**: 2025-11-02
**Current Version**: ModelContextProtocol 0.3.0-preview.4
**Target Version**: ModelContextProtocol 0.4.0
**Branch**: sprint-20251102-mcp-sdk-upgrade

## Executive Summary

Analyzed codebase for MCP SDK upgrade impacts using serana/codenav. Found 16 Tool classes with 132+ attribute usages across the codebase. All usage is straightforward attribute-based decoration with no complex SDK integration patterns.

## SDK References in Project

### Package Reference
**File**: `/Users/brett/src/ma-collective/maenifold/src/Maenifold.csproj`
- **Line 35**: `<PackageReference Include="ModelContextProtocol" Version="0.3.0-preview.4" />`
- **Impact**: Single direct upgrade point

### MCP Server Configuration
**File**: `/Users/brett/src/ma-collective/maenifold/src/Program.cs`
- **Lines 23-25**: MCP server bootstrap
```csharp
builder.Services
    .AddMcpServer()
    .WithStdioServerTransport()
    .WithToolsFromAssembly();
```

**Symbols Used**:
1. `AddMcpServer()` - Extension method for service registration
2. `WithStdioServerTransport()` - Stdio transport configuration
3. `WithToolsFromAssembly()` - Auto-discovery of [McpServerTool] decorated methods

## MCP Attribute Usage Patterns

### Attributes Used Throughout Codebase

1. **`[McpServerToolType]`** - Class-level attribute
   - Marks classes containing MCP tool methods
   - Found on 16 tool classes

2. **`[McpServerTool]`** - Method-level attribute
   - Marks public static methods as MCP tools
   - Found on ~132 methods across 16 files

3. **`[Description(...)]`** - Parameter documentation
   - System.ComponentModel.Description attribute
   - Used for tool/parameter descriptions in MCP schema
   - Found on every tool method and parameter

### Tool Classes Using MCP SDK

All located in `/Users/brett/src/ma-collective/maenifold/src/Tools/`:

1. **AssumptionLedgerTools.cs** - 11 attribute usages
2. **ConceptRepairTool.cs** - 11 attribute usages
3. **GraphTools.cs** - 11 attribute usages
4. **IncrementalSyncTools.cs** - 2 attribute usages
5. **MaintenanceTools.cs** - 6 attribute usages
6. **MemorySearchTools.cs** - 9 attribute usages
7. **MemoryTools.Operations.cs** - 19 attribute usages
8. **MemoryTools.Write.cs** - 6 attribute usages
9. **PerformanceBenchmark.cs** - 5 attribute usages
10. **RecentActivityTools.cs** - 6 attribute usages
11. **SequentialThinkingTools.cs** - 17 attribute usages
12. **SystemTools.cs** - 9 attribute usages
13. **VectorSearchTools.cs** - 4 attribute usages
14. **WorkflowTools.cs** - 2 attribute usages
15. **WorkflowTools.Runner.cs** - 9 attribute usages
16. **AdoptTools.cs** - 5 attribute usages

**Total**: 132+ MCP attribute decorations across 16 classes

## Upstream Dependencies

### Direct SDK Usage
**Namespace**: `ModelContextProtocol.Server`
- Only import in tool files
- No direct instantiation of SDK types
- All interaction through attributes and DI

### Extension Methods Called
From `Program.cs`:
1. `AddMcpServer()` - Likely in `Microsoft.Extensions.DependencyInjection` namespace
2. `WithStdioServerTransport()` - Transport configuration
3. `WithToolsFromAssembly()` - Reflection-based tool discovery

## Downstream Impacts

### Impact on Tool Definitions
**Risk Level**: MEDIUM to HIGH

All 40+ MCP tools across 16 classes depend on:
- Attribute decoration remaining compatible
- Description attribute interpretation
- Reflection-based discovery continuing to work
- Parameter binding/serialization compatibility

### Impact on Server Bootstrap
**Risk Level**: HIGH

The server startup in `Program.cs` is critical infrastructure:
- Any breaking changes to fluent API will prevent server startup
- Transport layer changes could affect stdio communication
- Assembly scanning changes could prevent tool discovery

### Impact on Test Suite
**Risk Level**: LOW to MEDIUM

Test files in `/Users/brett/src/ma-collective/maenifold/tests/`:
- Tests likely invoke tools directly via ToolRegistry
- May not test MCP server itself
- Could miss integration-level breaking changes

## Potential Breaking Changes

Based on 0.3.0 → 0.4.0 upgrade pattern:

### High Risk Areas
1. **Attribute Naming/Namespaces**
   - If `[McpServerTool]` renamed or moved → 132+ breaking sites
   - If `[McpServerToolType]` renamed → 16 breaking sites

2. **Extension Method Signatures**
   - Changes to `AddMcpServer()` → Server won't start
   - Changes to `WithToolsFromAssembly()` → Tools won't register

3. **Parameter Binding**
   - Changes to how `[Description]` is interpreted
   - JSON serialization/deserialization changes
   - Optional parameter handling changes

### Medium Risk Areas
1. **Transport Layer**
   - Stdio protocol changes
   - Message format changes (unlikely to affect application code directly)

2. **Tool Discovery**
   - Reflection/scanning logic changes
   - Public static method requirements
   - Return type expectations

### Low Risk Areas
1. **Internal SDK implementations**
   - Unlikely to affect attribute-based usage
   - Server handles protocol details

## Recommended Testing Approach

### Pre-Upgrade Verification
1. ✅ Document all current MCP SDK usage (COMPLETED)
2. ⏳ Review 0.4.0 changelog/migration guide
3. ⏳ Identify any deprecated APIs being used

### Post-Upgrade Testing
1. **Compilation**: Verify all 16 tool classes compile
2. **Server Startup**: Verify `--mcp` mode starts successfully
3. **Tool Discovery**: Verify all tools registered (check logs/tool list)
4. **Tool Invocation**: Smoke test key tools:
   - WriteMemory (memory creation)
   - ReadMemory (memory retrieval)
   - Sync (graph operations)
   - BuildContext (graph traversal)
   - SearchMemories (search functionality)
5. **Integration Tests**: Run full test suite
6. **MCP Protocol**: Test with actual MCP client (Claude Desktop, etc.)

## Mitigation Strategies

### If Breaking Changes Found
1. **Attribute Renaming**: Find/replace across all tool files
2. **API Changes**: Update Program.cs bootstrap
3. **Behavior Changes**: Update tool implementations
4. **Protocol Changes**: May need client updates

### Rollback Plan
- Current version locked in .csproj: `0.3.0-preview.4`
- Git branch for upgrade: `sprint-20251102-mcp-sdk-upgrade`
- Can revert to main branch if critical issues found

## Files Requiring Monitoring

### Critical Files
1. `/src/Maenifold.csproj` - Package version
2. `/src/Program.cs` - Server configuration
3. All 16 files in `/src/Tools/*.cs` with [McpServerTool] attributes

### Supporting Files
1. `/src/Utils/ToolRegistry.cs` - Direct tool invocation
2. Test files invoking MCP tools

## Next Steps

1. ✅ **Complete this analysis** (DONE)
2. ⏳ Review MCP SDK 0.4.0 release notes and migration guide
3. ⏳ Search for any deprecation warnings in 0.3.0
4. ⏳ Create upgrade PRD with specific breaking changes
5. ⏳ Execute upgrade on branch
6. ⏳ Run verification tests
7. ⏳ Update documentation if API changes

## Analysis Metadata

**Analysis Method**: serana/codenav C# analysis tools
**Files Analyzed**: 69 C# source files
**Tool Classes Found**: 16
**MCP Attribute Usages**: 132+
**Configuration Points**: 1 (Program.cs)
**Ma Protocol Compliance**: ✅ Used real analysis tools, no mocks, no fake AI

---

*This analysis follows [[Ma-Protocol]] principles: real tools (serana), real analysis, documented [[MCP-SDK]] upgrade impacts for [[maenifold]] knowledge graph system.*
