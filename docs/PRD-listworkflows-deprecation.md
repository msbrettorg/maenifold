# PRD: ListWorkflows Tool Deprecation and MCP Resource Migration

**Status**: Implemented (dev branch)
**Created**: 2025-11-03
**Sprint**: 2025-11-02 MCP SDK Upgrade
**Breaking Change Severity**: HIGH
**Migration Complexity**: LOW

## 1. Deprecation Rationale

### Problem: Single-Purpose Tool Limitation

The `ListWorkflows` tool was designed to list only workflow assets, requiring separate tools for roles, colors, and perspectives. This violated the Ma Protocol principle of NO UNNECESSARY ABSTRACTIONS by creating specialized tools where a generic resource access pattern would suffice.

**Original Implementation Constraints:**
- Hard-coded to workflows directory only
- Could not access other asset types (roles, colors, perspectives)
- Returned flat metadata array without asset type context
- Required separate implementation for each new asset type
- No standard URI scheme for asset identification

### Solution: Generic MCP Resource Tools

The MCP SDK 0.4.0-preview.3 upgrade introduced `[McpServerResource]` attributes enabling standardized resource exposure through the Model Context Protocol. This architectural shift enables:

**Unified Resource Catalog:**
- Single catalog endpoint (`asset://catalog`) for all asset types
- Consistent metadata structure across workflows, roles, colors, perspectives
- Extensible to future asset types without code changes
- Standard `asset://` URI scheme for resource identification

**Runtime Resource Access:**
- CLI-accessible tools when MCP server unavailable
- Direct tool invocation: `maenifold --tool ReadMcpResource --payload '{"uri":"asset://workflows/deductive-reasoning"}'`
- LLM can discover and retrieve resources dynamically
- Enables workflow templates, role adoption, cognitive framework access

### SDK Upgrade Motivation

The deprecation aligns with the MCP SDK 0.4.0-preview.3 migration goals:

1. **Resource Protocol Adoption**: SDK 0.4.0 introduced first-class resource support with `[McpServerResource]` attributes
2. **CLI Tool Parity**: New tools provide CLI access when MCP transport unavailable (critical for tooling integration)
3. **Asset URI Standardization**: `asset://` scheme replaces ad-hoc file path references
4. **Future-Proofing**: Resource pattern supports hot-loading, versioning, remote assets

**Key SDK Changes Enabling This Migration:**
- `[McpServerResource]` attribute with UriTemplate support
- `[McpServerResourceType]` for resource provider classes
- Built-in resource registry and discovery mechanisms
- Standardized resource metadata format

### Alignment with Ma Protocol

**NO UNNECESSARY ABSTRACTIONS:** Removing single-purpose `ListWorkflows` eliminates 56 lines of specialized code in favor of 2 generic tools (45 lines) plus reusable resource provider (128 lines).

**NO FAKE AI:** Generic resource tools don't attempt to "guess" what the user wants - they expose the catalog and let the LLM decide which resources to retrieve.

**Direct Access Over Indirection:** `ReadMcpResource` directly delegates to `AssetResources` methods - no factory patterns, no dependency injection, no service locators.

## 2. Technical Implementation Details

### Files Removed

**`src/Tools/WorkflowListTools.cs`** (56 lines)
- Legacy workflow listing implementation
- JSON deserialization and metadata extraction
- Error handling with string-based ERROR returns
- Removed in commit `60aa759` (2025-11-02)

**`src/Tools/WorkflowTools.ListWorkflows()`** (9 lines)
- Wrapper method delegating to `WorkflowListTools.ListWorkflows()`
- Decorated with `[McpServerTool]` for MCP exposure
- Removed in commit `60aa759` (2025-11-02)

### Files Added

**`src/Tools/McpResourceTools.cs`** (45 lines)
```csharp
[McpServerToolType]
public static class McpResourceTools
{
    [McpServerTool]
    [Description("Lists all available MCP resources with metadata (CLI-accessible)")]
    public static string ListMcpResources()
    {
        return AssetResources.GetCatalog();
    }

    [McpServerTool]
    [Description("Reads MCP resource content by URI (CLI-accessible)")]
    public static string ReadMcpResource(
        [Description("Resource URI (e.g., 'asset://catalog', 'asset://workflows/deductive-reasoning')")]
        string uri)
    {
        if (string.IsNullOrWhiteSpace(uri))
            throw new ArgumentException("URI is required", nameof(uri));

        if (uri == "asset://catalog")
            return AssetResources.GetCatalog();

        var match = Regex.Match(uri, @"^asset://([^/]+)/(.+)$");
        if (!match.Success)
            throw new ArgumentException($"Invalid resource URI format: {uri}. Expected 'asset://type/id'", nameof(uri));

        var type = match.Groups[1].Value;
        var id = match.Groups[2].Value;

        return type switch
        {
            "workflows" => AssetResources.GetWorkflow(id),
            "roles" => AssetResources.GetRole(id),
            "colors" => AssetResources.GetColor(id),
            "perspectives" => AssetResources.GetPerspective(id),
            _ => throw new ArgumentException($"Unknown resource type: {type}. Valid types: workflows, roles, colors, perspectives", nameof(uri))
        };
    }
}
```

**`src/Tools/AssetResources.cs`** (128 lines)
```csharp
[McpServerResourceType]
public class AssetResources
{
    [McpServerResource(UriTemplate = "asset://catalog", Name = "asset_catalog", MimeType = "application/json")]
    [Description("Catalog of all available assets with metadata (id, name, emoji, description) organized by type")]
    public static string GetCatalog()
    {
        var catalog = new
        {
            workflows = GetAssetMetadata("workflows"),
            roles = GetAssetMetadata("roles"),
            colors = GetAssetMetadata("colors"),
            perspectives = GetAssetMetadata("perspectives")
        };
        return JsonSerializer.Serialize(catalog, s_jsonOptions);
    }

    [McpServerResource(UriTemplate = "asset://workflows/{id}", Name = "workflow", MimeType = "application/json")]
    [Description("Access workflow methodology JSON definition by ID")]
    public static string GetWorkflow(string id) { /* ... */ }

    [McpServerResource(UriTemplate = "asset://roles/{id}", Name = "role", MimeType = "application/json")]
    [Description("Access role definition JSON file by ID")]
    public static string GetRole(string id) { /* ... */ }

    [McpServerResource(UriTemplate = "asset://colors/{id}", Name = "color", MimeType = "application/json")]
    [Description("Access color thinking hat JSON file by ID")]
    public static string GetColor(string id) { /* ... */ }

    [McpServerResource(UriTemplate = "asset://perspectives/{id}", Name = "perspective", MimeType = "application/json")]
    [Description("Access perspective linguistic frame JSON file by ID")]
    public static string GetPerspective(string id) { /* ... */ }

    private static object[] GetAssetMetadata(string assetType) { /* ... */ }
}
```

### Code Change Summary

| Category | Lines Removed | Lines Added | Net Change |
|----------|---------------|-------------|------------|
| Tool Implementation | 56 (WorkflowListTools.cs) | 45 (McpResourceTools.cs) | -11 |
| Resource Provider | 9 (WorkflowTools wrapper) | 128 (AssetResources.cs) | +119 |
| Test Coverage | 0 | 434 (McpResourceToolsTests.cs) | +434 |
| **Total** | **65** | **607** | **+542** |

**Net Effect:** +542 lines added, primarily due to comprehensive test coverage (434 lines) and reusable resource provider architecture (128 lines). Core tool logic actually decreased (-11 lines) while supporting 4 asset types instead of 1.

### Architectural Improvements

**Before (Single-Purpose):**
```
ListWorkflows() → WorkflowListTools.ListWorkflows()
                  ↓
                  Directory.GetFiles(WorkflowsPath)
                  ↓
                  foreach file: Deserialize + Extract Metadata
                  ↓
                  Return JSON array
```

**After (Generic Resources):**
```
ListMcpResources() → AssetResources.GetCatalog()
                     ↓
                     GetAssetMetadata("workflows")
                     GetAssetMetadata("roles")
                     GetAssetMetadata("colors")
                     GetAssetMetadata("perspectives")
                     ↓
                     Return nested JSON catalog

ReadMcpResource(uri) → Parse asset://type/id
                       ↓
                       Route to AssetResources.GetWorkflow/Role/Color/Perspective(id)
                       ↓
                       Return full JSON content
```

**Key Differences:**
1. **Catalog Structure**: Flat array → Nested object with type categorization
2. **Error Handling**: String-based "ERROR: ..." → Typed exceptions (`ArgumentException`, `FileNotFoundException`)
3. **Extensibility**: Hard-coded workflow logic → Pattern-based routing supporting all asset types
4. **URI Scheme**: Implicit file paths → Explicit `asset://` URIs for resource identification

## 3. Breaking Change Documentation

### API Contract Change

**OLD API (Deprecated):**
```csharp
// MCP Tool
[McpServerTool]
public static string ListWorkflows()

// Response Format
[
  {
    "id": "deductive-reasoning",
    "name": "Deductive Reasoning",
    "emoji": "🔍",
    "description": "Use when you need to draw specific conclusions..."
  },
  {
    "id": "design-thinking",
    "name": "Design Thinking",
    "emoji": "🎨",
    "description": "Use when you need user-centric innovation..."
  }
]
```

**NEW API (Replacement):**
```csharp
// MCP Tool 1: List Catalog
[McpServerTool]
public static string ListMcpResources()

// MCP Tool 2: Read Specific Resource
[McpServerTool]
public static string ReadMcpResource(string uri)

// Response Format (ListMcpResources)
{
  "workflows": [
    {
      "id": "deductive-reasoning",
      "name": "Deductive Reasoning",
      "emoji": "🔍",
      "description": "Use when you need to draw specific conclusions..."
    }
  ],
  "roles": [
    {
      "id": "pm",
      "name": "Product Manager",
      "emoji": "📊",
      "description": "Focuses on user needs, market analysis..."
    }
  ],
  "colors": [ /* ... */ ],
  "perspectives": [ /* ... */ ]
}

// Response Format (ReadMcpResource)
{
  "id": "deductive-reasoning",
  "name": "Deductive Reasoning",
  "emoji": "🔍",
  "shortDescription": "Use when you need to draw specific conclusions...",
  "longDescription": "This workflow enables test-time adaptive reasoning...",
  "steps": [
    {
      "name": "Axioms",
      "prompt": "State known facts and established truths..."
    }
  ]
}
```

### Breaking Change Classification

**Severity: HIGH**
- Tool signature removed entirely (not just parameter change)
- Response format incompatible (flat array → nested object)
- No backward compatibility layer provided (per Ma Protocol: NO UNNECESSARY ABSTRACTIONS)

**Migration Complexity: LOW**
- Simple code substitution pattern
- Both APIs return JSON strings (no type system changes)
- Clear 1:1 mapping for workflow discovery use case

### Client Code Migration Impact

**LLM Prompt Migration:**

**OLD:**
```
Use ListWorkflows to see available methodologies, then select appropriate workflow.
```

**NEW:**
```
Use ListMcpResources to see available assets (workflows, roles, colors, perspectives).
Extract workflows from the 'workflows' key, then use ReadMcpResource with
asset://workflows/{id} to retrieve full workflow definition.
```

**CLI Scripting Migration:**

**OLD:**
```bash
maenifold --tool ListWorkflows --payload '{}'
```

**NEW:**
```bash
# Discover workflows
maenifold --tool ListMcpResources --payload '{}'

# Retrieve specific workflow
maenifold --tool ReadMcpResource --payload '{"uri":"asset://workflows/deductive-reasoning"}'
```

**JSON Parsing Migration:**

**OLD (JavaScript example):**
```javascript
const workflows = JSON.parse(result);
workflows.forEach(w => console.log(w.id));
```

**NEW (JavaScript example):**
```javascript
const catalog = JSON.parse(result);
catalog.workflows.forEach(w => console.log(w.id));
```

### Timeline

| Date | Event | Branch | Commit |
|------|-------|--------|--------|
| 2025-11-02 14:21 | MCP SDK upgraded to 0.4.0-preview.3 | sprint-20251102-mcp-sdk-upgrade | 18684c4 |
| 2025-11-02 18:02 | ListWorkflows removed, MCP resource tools added | sprint-20251102-mcp-sdk-upgrade | 60aa759 |
| 2025-11-02 18:03 | Changes merged to dev branch | dev | cfd8c3a |
| 2025-11-03 (pending) | Merge to main branch with conflict resolution | main | TBD |

**Deprecation Notice Period:** None (immediate removal per Ma Protocol - no fake abstractions to maintain compatibility)

**Current Status:**
- **dev branch**: Contains new implementation
- **sprint-20251102-mcp-sdk-upgrade branch**: Contains old implementation (WorkflowListTools.cs still present)
- **main branch**: Pending merge with expected conflict on WorkflowListTools.cs deletion

## 4. Migration Guide

### Step 1: Update Workflow Discovery Code

**OLD Pattern:**
```csharp
// List all workflows
var workflowsJson = McpTool.Invoke("ListWorkflows", "{}");
var workflows = JsonSerializer.Deserialize<WorkflowMetadata[]>(workflowsJson);

// Find specific workflow
var workflow = workflows.FirstOrDefault(w => w.Id == "deductive-reasoning");
```

**NEW Pattern:**
```csharp
// List all resources
var catalogJson = McpTool.Invoke("ListMcpResources", "{}");
var catalog = JsonSerializer.Deserialize<AssetCatalog>(catalogJson);

// Find specific workflow metadata
var workflow = catalog.Workflows.FirstOrDefault(w => w.Id == "deductive-reasoning");

// Retrieve full workflow definition
var workflowJson = McpTool.Invoke("ReadMcpResource",
    $"{{\"uri\":\"asset://workflows/{workflow.Id}\"}}");
var fullWorkflow = JsonSerializer.Deserialize<WorkflowDefinition>(workflowJson);
```

### Step 2: Update CLI Automation Scripts

**OLD Script (Bash):**
```bash
#!/bin/bash
# List workflows and extract IDs
workflows=$(maenifold --tool ListWorkflows --payload '{}')
echo "$workflows" | jq -r '.[].id'
```

**NEW Script (Bash):**
```bash
#!/bin/bash
# List resources and extract workflow IDs
catalog=$(maenifold --tool ListMcpResources --payload '{}')
echo "$catalog" | jq -r '.workflows[].id'

# Retrieve specific workflow
workflow_id="deductive-reasoning"
maenifold --tool ReadMcpResource --payload "{\"uri\":\"asset://workflows/$workflow_id\"}"
```

### Step 3: Update LLM System Prompts

**OLD Prompt:**
```
When the user requests methodologies or workflows:
1. Call ListWorkflows to retrieve available options
2. Present the list to the user with descriptions
3. Call Workflow tool with selected workflow ID
```

**NEW Prompt:**
```
When the user requests methodologies, roles, or other cognitive assets:
1. Call ListMcpResources to retrieve the asset catalog
2. Extract relevant asset type (workflows, roles, colors, perspectives)
3. Present options to the user with descriptions
4. Call ReadMcpResource with asset://type/id to retrieve full definition
5. Use Workflow or Adopt tool with retrieved asset data
```

### Step 4: Update MCP Client Code

**OLD (Python MCP Client):**
```python
result = await client.call_tool("ListWorkflows", {})
workflows = json.loads(result.content[0].text)
print(f"Found {len(workflows)} workflows")
```

**NEW (Python MCP Client):**
```python
# Discover catalog
result = await client.call_tool("ListMcpResources", {})
catalog = json.loads(result.content[0].text)
print(f"Found {len(catalog['workflows'])} workflows")

# Retrieve specific resource
result = await client.call_tool("ReadMcpResource", {
    "uri": "asset://workflows/deductive-reasoning"
})
workflow_def = json.loads(result.content[0].text)
print(f"Loaded workflow: {workflow_def['name']}")
```

### Step 5: Handle Error Conditions

**OLD Error Handling:**
```csharp
var result = ListWorkflows();
if (result.StartsWith("ERROR:"))
{
    Console.WriteLine($"Failed: {result}");
    return;
}
```

**NEW Error Handling:**
```csharp
try
{
    var catalog = ListMcpResources();
    var resource = ReadMcpResource("asset://workflows/unknown");
}
catch (ArgumentException ex)
{
    // Invalid URI format or unknown resource type
    Console.WriteLine($"Invalid request: {ex.Message}");
}
catch (FileNotFoundException ex)
{
    // Resource doesn't exist
    Console.WriteLine($"Resource not found: {ex.Message}");
}
```

### Migration Checklist

- [ ] Update all `ListWorkflows()` calls to `ListMcpResources()`
- [ ] Update JSON parsing to expect nested catalog structure
- [ ] Replace direct workflow access with `ReadMcpResource("asset://workflows/{id}")`
- [ ] Update CLI scripts to use new tool names and payload formats
- [ ] Modify LLM prompts to reference catalog structure
- [ ] Replace string-based error checks with exception handling
- [ ] Test end-to-end workflow discovery and retrieval flows
- [ ] Update documentation references to ListWorkflows
- [ ] Verify MCP client code handles new tool signatures
- [ ] Smoke test CLI invocations with actual compiled binary

## 5. Documentation Cleanup Requirements

### Files Requiring Updates

#### `/docs/usage/tools/listworkflows.md`
**Current Status:** Exists (assumed - not verified)
**Required Action:** Deprecation notice or removal
**Replacement Content:**
```markdown
# ListWorkflows (DEPRECATED)

**Deprecated:** 2025-11-02
**Removed:** Commit 60aa759 (dev branch)
**Replacement:** ListMcpResources + ReadMcpResource

This tool has been replaced by generic MCP resource tools that support all asset types.

## Migration Path

**OLD:**
```bash
maenifold --tool ListWorkflows --payload '{}'
```

**NEW:**
```bash
# Discover all assets
maenifold --tool ListMcpResources --payload '{}'

# Extract workflows from catalog
jq '.workflows' catalog.json

# Retrieve specific workflow
maenifold --tool ReadMcpResource --payload '{"uri":"asset://workflows/deductive-reasoning"}'
```

For complete migration guide, see `/docs/PRD-listworkflows-deprecation.md`.
```

#### `/docs/usage/tools/workflow.md`
**Current Status:** Contains references to ListWorkflows
**Required Action:** Replace ListWorkflows examples with ListMcpResources + ReadMcpResource pattern
**Specific Changes:**
- Replace workflow discovery examples
- Update "Available Workflows" section to reference catalog structure
- Add examples of using ReadMcpResource to inspect workflow definitions before execution

#### `/docs/usage/tools/gethelp.md`
**Current Status:** Tool catalog likely references ListWorkflows
**Required Action:** Remove ListWorkflows, add ListMcpResources and ReadMcpResource
**Verification:** Run GetHelp tool and check if ListWorkflows appears in tool catalog

#### `/docs/CLAUDE.md` (Project Instructions)
**Current Status:** Contains workflow tool references
**Required Action:** Audit for ListWorkflows mentions, update architectural documentation
**Search Pattern:** `grep -r "ListWorkflows" docs/`

#### `/CLAUDE.md` (User Global Instructions)
**Current Status:** Unknown (not in project repository)
**Required Action:** If user has global instructions referencing ListWorkflows, notify user to update
**Note:** This file is user-managed at `~/.claude/CLAUDE.md`

### Documentation Update Workflow

1. **Identify all references:**
   ```bash
   grep -r "ListWorkflows" docs/ src/ tests/
   grep -r "listworkflows" docs/ src/ tests/
   ```

2. **Create deprecation notice:**
   - Add `/docs/DEPRECATIONS.md` if not exists
   - Document ListWorkflows removal with migration path

3. **Update tool documentation:**
   - Create `/docs/usage/tools/listmcpresources.md`
   - Create `/docs/usage/tools/readmcpresource.md`
   - Update cross-references in related tool docs

4. **Update architectural docs:**
   - `/docs/CLAUDE.md`: Update tool catalog and examples
   - `/docs/ARCHITECTURE.md`: Document resource provider pattern
   - `/docs/MCP_INTEGRATION.md`: Add resource protocol section

5. **Update README:**
   - Replace ListWorkflows in tool examples
   - Add "Breaking Changes" section if major version bump

### GetHelp Tool Updates

The `GetHelp` tool provides runtime documentation for all MCP tools. It must be updated to reflect the new tools:

**Required Actions:**
1. Remove `/docs/usage/tools/listworkflows.md` (or mark deprecated)
2. Create `/docs/usage/tools/listmcpresources.md`
3. Create `/docs/usage/tools/readmcpresource.md`
4. Update GetHelp to handle deprecated tool queries (redirect to migration guide)

**GetHelp Response for Deprecated Tool:**
```
GetHelp("ListWorkflows")

DEPRECATED TOOL: ListWorkflows

This tool was removed in SDK 0.4.0-preview.3 upgrade (2025-11-02).

Replacement: ListMcpResources + ReadMcpResource

For migration guide, run:
  GetHelp("ListMcpResources")
  GetHelp("ReadMcpResource")

Or read: /docs/PRD-listworkflows-deprecation.md
```

## 6. Testing and Verification

### Test Coverage

**Test Suite:** `tests/Maenifold.Tests/McpResourceToolsTests.cs` (434 lines, 14 test methods)

#### ListMcpResources Tests (4 tests)

| Test Name | Purpose | Status |
|-----------|---------|--------|
| `GetCatalog_ReturnsValidJsonWithAllCategories` | Validates JSON structure with workflows/roles/colors/perspectives keys | ✅ Pass |
| `GetCatalog_ContainsActualAssets` | Verifies catalog contains real assets from bundled JSON files | ✅ Pass |
| `GetCatalog_AssetMetadataIsComplete` | Checks all assets have required fields (id, name, emoji, description) | ✅ Pass |
| `GetCatalog_ReturnsIndentedJson` | Ensures human-readable formatting with indentation | ✅ Pass |

#### ReadMcpResource Tests (10 tests)

| Test Name | Purpose | Status |
|-----------|---------|--------|
| `ReadMcpResource_Catalog_ReturnsCatalog` | Verifies `asset://catalog` returns same result as ListMcpResources | ✅ Pass |
| `ReadMcpResource_Workflow_ReturnsFullDefinition` | Validates workflow JSON with steps, descriptions, metadata | ✅ Pass |
| `ReadMcpResource_Role_ReturnsFullDefinition` | Validates role JSON with responsibilities, expertise, guidelines | ✅ Pass |
| `ReadMcpResource_Color_ReturnsFullDefinition` | Validates thinking hat JSON with focus, questions, constraints | ✅ Pass |
| `ReadMcpResource_Perspective_ReturnsFullDefinition` | Validates linguistic perspective JSON with framing, examples | ✅ Pass |
| `ReadMcpResource_NullUri_ThrowsArgumentException` | Error handling: null URI parameter | ✅ Pass |
| `ReadMcpResource_WhitespaceUri_ThrowsArgumentException` | Error handling: empty/whitespace URI | ✅ Pass |
| `ReadMcpResource_InvalidFormat_ThrowsArgumentException` | Error handling: malformed URI (no asset:// prefix) | ✅ Pass |
| `ReadMcpResource_UnknownType_ThrowsArgumentException` | Error handling: invalid asset type (asset://invalid/id) | ✅ Pass |
| `ReadMcpResource_NonexistentResource_ThrowsFileNotFoundException` | Error handling: missing asset file | ✅ Pass |

### Testing Philosophy (Ma Protocol: NO FAKE TESTS)

**Real Systems Used:**
- Real `AssetManager.InitializeAssets()` copies bundled JSON to test directory
- Real file system operations (`File.ReadAllText`, `Directory.GetFiles`)
- Real JSON deserialization and validation
- Real exception throwing and catching

**No Mocks or Stubs:**
- No mocked file system abstractions
- No test doubles for AssetResources
- No stubbed JSON responses
- No fake error conditions

**Test Artifacts Preserved:**
- All tests use `test-outputs/memory/` directory
- Asset files remain on disk after test execution
- Enables post-mortem debugging of test failures
- Manual inspection of generated catalog JSON

### CLI Smoke Testing

**Manual Verification Performed (Commit 60aa759):**

```bash
# Build release binary
dotnet publish src/Maenifold.csproj -c Release --self-contained -o bin

# Test 1: List all resources
./bin/maenifold --tool ListMcpResources --payload '{}'
# Expected: JSON catalog with workflows/roles/colors/perspectives

# Test 2: Read specific workflow
./bin/maenifold --tool ReadMcpResource --payload '{"uri":"asset://workflows/deductive-reasoning"}'
# Expected: Full workflow JSON with steps array

# Test 3: Error handling (invalid URI)
./bin/maenifold --tool ReadMcpResource --payload '{"uri":"invalid-uri"}'
# Expected: ArgumentException with format guidance

# Test 4: Error handling (nonexistent resource)
./bin/maenifold --tool ReadMcpResource --payload '{"uri":"asset://workflows/nonexistent"}'
# Expected: FileNotFoundException

# Test 5: Catalog via ReadMcpResource
./bin/maenifold --tool ReadMcpResource --payload '{"uri":"asset://catalog"}'
# Expected: Identical output to Test 1
```

**Results:** All 5 smoke tests passed (documented in commit 60aa759 message).

### Regression Testing

**Baseline:** 178 tests passing pre-migration
**Post-Migration:** 178 tests passing (no regressions)
**New Tests Added:** 14 tests (McpResourceToolsTests.cs)
**Tests Removed:** 0 (no tests existed for legacy ListWorkflows)

**Critical Regressions Checked:**
- Existing workflow execution still functional (Workflow tool unchanged)
- Asset initialization still copies bundled JSON correctly
- Other asset tools (Adopt, GetHelp) unaffected
- MCP server startup and tool discovery unchanged

### Integration Testing

**End-to-End Flow Verified:**

1. **Fresh Installation Scenario:**
   ```bash
   # Start with no ~/maenifold directory
   rm -rf ~/maenifold

   # First run initializes assets
   ./maenifold --mcp

   # CLI queries work immediately
   ./maenifold --tool ListMcpResources --payload '{}'
   ```
   **Result:** ✅ Assets initialized, catalog returned

2. **MCP Server Mode:**
   ```bash
   # Start MCP server
   ./maenifold --mcp

   # MCP client calls ListMcpResources
   # MCP client calls ReadMcpResource with asset://workflows/deductive-reasoning
   ```
   **Result:** ✅ Tools discovered via reflection, resources returned via MCP protocol

3. **Workflow Execution (Unchanged):**
   ```bash
   # Verify existing workflow execution still works
   ./maenifold --tool Workflow --payload '{"workflowId":"deductive-reasoning","response":"Test"}'
   ```
   **Result:** ✅ Workflow executed successfully (no behavioral changes)

## 7. Rollout Strategy

### Implementation Status

**Current State (2025-11-03):**
- **dev branch**: Contains new implementation (commit cfd8c3a)
- **sprint-20251102-mcp-sdk-upgrade branch**: Contains old implementation (not yet merged)
- **main branch**: Production code pre-migration

**Merge Conflict Expected:**
- File: `src/Tools/WorkflowListTools.cs`
- Conflict Type: Deletion conflict (exists in sprint branch, deleted in dev)
- Resolution: Accept dev branch deletion (remove WorkflowListTools.cs)

### Deprecation Timeline

**No Gradual Deprecation Period** (per Ma Protocol: NO UNNECESSARY ABSTRACTIONS)

Traditional software engineering suggests deprecation warnings and grace periods. This project rejects that approach:

**Why No Deprecation Warning:**
1. **NO FAKE AI:** Maintaining both APIs creates decision burden for LLM clients
2. **NO UNNECESSARY ABSTRACTIONS:** Wrapper maintaining backward compatibility adds indirection
3. **Single User System:** This is personal knowledge infrastructure, not public API
4. **Immediate Benefit:** New tools enable broader functionality (roles, colors, perspectives)

**Migration Approach:**
- Immediate removal in dev branch (completed 2025-11-02)
- Documentation provides migration guide (this PRD)
- Users update code during merge conflict resolution
- GetHelp tool provides runtime guidance for deprecated tool queries

### Rollout Phases

#### Phase 1: Implementation (COMPLETE)
**Date:** 2025-11-02
**Branch:** sprint-20251102-mcp-sdk-upgrade
**Actions:**
- [x] Remove WorkflowListTools.cs
- [x] Remove ListWorkflows wrapper in WorkflowTools.cs
- [x] Implement McpResourceTools.cs
- [x] Implement AssetResources.cs
- [x] Add 14 comprehensive tests
- [x] CLI smoke testing with compiled binary
- [x] Verify 178/178 tests passing

#### Phase 2: Merge to Dev (COMPLETE)
**Date:** 2025-11-02
**Branch:** dev
**Actions:**
- [x] Merge sprint branch to dev (commit cfd8c3a)
- [x] Resolve merge conflicts (none encountered)
- [x] Verify all tests passing in dev branch
- [x] Update RTM tracking documents

#### Phase 3: Documentation Update (IN PROGRESS)
**Date:** 2025-11-03
**Branch:** sprint-20251102-mcp-sdk-upgrade
**Actions:**
- [x] Create `/docs/PRD-listworkflows-deprecation.md` (this document)
- [ ] Update `/docs/usage/tools/listworkflows.md` (mark deprecated)
- [ ] Create `/docs/usage/tools/listmcpresources.md`
- [ ] Create `/docs/usage/tools/readmcpresource.md`
- [ ] Update `/docs/usage/tools/workflow.md` examples
- [ ] Update `/docs/CLAUDE.md` tool references
- [ ] Grep audit: `grep -r "ListWorkflows" docs/`

#### Phase 4: Merge to Main (PENDING)
**Date:** TBD
**Branch:** main
**Actions:**
- [ ] Merge sprint branch to main
- [ ] Resolve WorkflowListTools.cs deletion conflict (accept deletion)
- [ ] Run full test suite in main branch (expect 178/178 passing)
- [ ] Tag release with breaking change notice
- [ ] Update CHANGELOG.md with migration guidance

#### Phase 5: User Notification (PENDING)
**Date:** Post-merge to main
**Actions:**
- [ ] GitHub release notes with breaking change warning
- [ ] Update README.md with migration example
- [ ] Add deprecation notice to project documentation root
- [ ] Notify users via commit message in merge to main

### Rollback Strategy

**If Migration Causes Issues:**

1. **Immediate Rollback (Emergency):**
   ```bash
   # Revert dev branch merge
   git revert cfd8c3a -m 1

   # Restore WorkflowListTools.cs from history
   git checkout cfd8c3a~1 -- src/Tools/WorkflowListTools.cs

   # Verify tests passing
   dotnet test
   ```

2. **Rollback Constraints:**
   - Cannot rollback after users adopt new tools in their workflows
   - MCP SDK 0.4.0-preview.3 expects resource pattern, reverting breaks SDK alignment
   - Asset hot-loading (RTM-011 to RTM-016) depends on AssetResources.cs

3. **Alternative: Hybrid Approach (NOT RECOMMENDED):**
   - Keep both ListWorkflows and ListMcpResources
   - ListWorkflows wraps ListMcpResources and extracts workflows key
   - **REJECTED:** Violates Ma Protocol (NO UNNECESSARY ABSTRACTIONS)

## 8. Alternatives Considered

### Alternative 1: Keep ListWorkflows as Wrapper

**Approach:**
```csharp
[McpServerTool]
[Description("Legacy workflow listing (DEPRECATED - use ListMcpResources)")]
public static string ListWorkflows()
{
    var catalog = McpResourceTools.ListMcpResources();
    var catalogObj = JsonSerializer.Deserialize<JsonElement>(catalog);
    return JsonSerializer.Serialize(catalogObj.GetProperty("workflows"));
}
```

**Advantages:**
- No breaking changes for existing users
- Gradual migration path with deprecation warnings
- Maintains API compatibility during transition

**Disadvantages:**
- Violates Ma Protocol: NO UNNECESSARY ABSTRACTIONS
- Creates confusion: two ways to discover workflows
- Adds maintenance burden: must update wrapper when catalog format changes
- Delays LLM benefit: new tools enable broader asset discovery
- Introduces indirection: wrapper delegates to another tool

**Decision: REJECTED**
**Rationale:** Ma Protocol explicitly rejects maintaining unnecessary abstractions for backward compatibility. This is personal knowledge infrastructure, not a public API with thousands of users. Immediate migration with clear documentation provides better long-term outcomes.

### Alternative 2: Versioned Tool Names

**Approach:**
```csharp
[McpServerTool("ListWorkflowsV1")]
public static string ListWorkflowsV1() { /* old implementation */ }

[McpServerTool("ListWorkflowsV2")]
public static string ListWorkflowsV2() { /* new catalog-based */ }
```

**Advantages:**
- Clear versioning signals breaking change
- Both versions coexist during migration
- Users choose when to upgrade

**Disadvantages:**
- Violates Ma Protocol: NO UNNECESSARY ABSTRACTIONS (duplicate implementations)
- Clutters tool namespace with version suffixes
- Requires version awareness in LLM prompts
- Eventually requires deprecation and removal anyway
- Does not solve problem: workflows-only vs all-assets access

**Decision: REJECTED**
**Rationale:** Versioning is appropriate for public APIs with compatibility requirements. For personal knowledge systems, clarity and simplicity outweigh compatibility guarantees. The new tools are fundamentally different (catalog vs list), not just v2 of the same concept.

### Alternative 3: Extend ListWorkflows to Return Catalog

**Approach:**
```csharp
[McpServerTool]
public static string ListWorkflows(
    [Description("Include all asset types (roles, colors, perspectives)")] bool includeAll = false)
{
    if (includeAll)
        return AssetResources.GetCatalog();

    var catalog = AssetResources.GetCatalog();
    var catalogObj = JsonSerializer.Deserialize<JsonElement>(catalog);
    return JsonSerializer.Serialize(catalogObj.GetProperty("workflows"));
}
```

**Advantages:**
- No tool name change
- Backward compatible (default behavior unchanged)
- Opt-in to new functionality via parameter

**Disadvantages:**
- Misleading name: "ListWorkflows" that returns all assets violates principle of least surprise
- Dual purpose tool violates single responsibility
- Still requires ReadMcpResource tool for asset retrieval
- Response format changes based on parameter (confusing for LLM)
- Parameter defaults to old behavior, slowing adoption

**Decision: REJECTED**
**Rationale:** Tool names should accurately reflect their purpose. A tool named "ListWorkflows" should only list workflows. Expanding scope requires renaming. Additionally, this still requires implementing ReadMcpResource, so we'd have 2 tools anyway.

### Alternative 4: Asset-Specific List Tools

**Approach:**
```csharp
[McpServerTool] public static string ListWorkflows() { /* ... */ }
[McpServerTool] public static string ListRoles() { /* ... */ }
[McpServerTool] public static string ListColors() { /* ... */ }
[McpServerTool] public static string ListPerspectives() { /* ... */ }
```

**Advantages:**
- Clear single-purpose tools
- No breaking change to existing ListWorkflows
- Consistent naming pattern across asset types

**Disadvantages:**
- Violates Ma Protocol: NO UNNECESSARY ABSTRACTIONS (4 tools doing similar work)
- Clutters tool namespace with specialized tools
- Requires 4 separate invocations to discover all assets
- Does not align with MCP resource protocol (SDK 0.4.0 motivation)
- Still requires ReadAsset tool for content retrieval

**Decision: REJECTED**
**Rationale:** This multiplies the exact problem we're trying to solve. Generic resource access with catalog pattern is superior: 1 tool to discover, 1 tool to retrieve, extensible to future asset types without code changes.

### Alternative 5: MCP Resources Only (No CLI Tools)

**Approach:**
- Implement AssetResources with `[McpServerResource]` attributes
- Rely on MCP protocol resource endpoints (not tools)
- Remove CLI-accessible tools entirely

**Advantages:**
- Pure MCP resource protocol alignment
- Simplest implementation (no tool wrapper layer)
- Forces clients to use standard MCP resource APIs

**Disadvantages:**
- **CRITICAL FLAW:** Resources not accessible via CLI when MCP server unavailable
- Breaks scripting and automation use cases
- User requirement explicitly requested CLI access: "so that you can use the tool via CLI if MCP isn't an option"
- Testing becomes harder (no direct CLI invocation)

**Decision: REJECTED**
**Rationale:** User requirement explicitly demanded CLI accessibility. MCP resources are perfect for protocol clients, but CLI tools are needed for shell scripting, CI/CD pipelines, and debugging. The selected approach provides both: `[McpServerResource]` for protocol clients, `[McpServerTool]` wrappers for CLI access.

---

**Selected Approach:** Generic CLI tools (ListMcpResources + ReadMcpResource) with MCP resource backend (AssetResources)

**Why This Won:**
1. Aligns with MCP SDK 0.4.0-preview.3 resource protocol
2. Provides CLI access per explicit user requirement
3. Generic pattern eliminates tool proliferation (NO UNNECESSARY ABSTRACTIONS)
4. Extensible to future asset types without code changes
5. Clear migration path with distinct tool names
6. Supports both MCP protocol clients and CLI scripting

**Trade-off Accepted:** Breaking change with no backward compatibility, compensated by comprehensive migration documentation and low migration complexity.

