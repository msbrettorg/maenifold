# Sprint Specifications: MCP SDK Upgrade

**Sprint ID**: `sprint-20251102-mcp-sdk-upgrade`
**Branch**: `sprint-20251102-mcp-sdk-upgrade`
**Workflow Session**: `workflow-1762117950582`
**Specifications Session**: `session-1762118296713`

---

## Functional Requirements

### FR-1: MCP SDK Version Upgrade
**Priority**: MUST HAVE
**Description**: Upgrade ModelContextProtocol SDK from 0.3.0-preview.4 to 0.4.0-preview.3

**Implementation Details**:
- Update `src/Maenifold.csproj:35` package reference
- Verify MCP server builder chain in `src/Program.cs:23-25` compiles
- Ensure all 40 existing MCP tools remain functional after upgrade
- Breaking changes from 0.3.0 to 0.4.0 are acceptable (no backward compatibility required)

**Affected Files**:
- src/Maenifold.csproj
- src/Program.cs (MCP server initialization)
- All files in src/Tools/ (40 tool implementations)

---

### FR-2: Asset Hot-Loading via MCP Resources
**Priority**: MUST HAVE
**Description**: Expose cognitive assets (workflows, roles, colors, perspectives) as MCP resources with automatic refresh on file changes

**Implementation Details**:
- Implement attribute-based resource pattern using SDK 0.4.0 `[McpServerResource]`
- AssetManager exposes 28 workflow JSON files as MCP resources
- AssetManager exposes 7 role JSON files as MCP resources
- AssetManager exposes 7 color JSON files as MCP resources
- AssetManager exposes 12 perspective JSON files as MCP resources
- Total: 54 asset files exposed as dynamic MCP resources

**Affected Files**:
- src/Utils/AssetManager.cs (new resource methods)
- src/assets/workflows/*.json (28 files)
- src/assets/roles/*.json (7 files)
- src/assets/colors/*.json (7 files)
- src/assets/perspectives/*.json (12 files)

---

### FR-3: FileSystemWatcher for Asset Changes
**Priority**: MUST HAVE
**Description**: Monitor ~/maenifold/assets/**/*.json for file creation, modification, and deletion

**Implementation Details**:
- Reuse IncrementalSyncTools.cs FileSystemWatcher pattern
- Monitor filter: `*.json` in assets directory and subdirectories
- Debounce delay: 150ms (Config.DefaultDebounceMs)
- NotifyFilter: FileName | LastWrite
- Buffer size: 64KB (Config.WatcherBufferSize)
- Events: Created, Changed, Deleted, Renamed, Error

**Reference Implementation**: src/Tools/IncrementalSyncTools.cs:22-82

---

### FR-4: Resource Update Notifications
**Priority**: MUST HAVE
**Description**: When asset files change, notify MCP client to refresh resource list

**Implementation Details**:
- OnFileCreated → Add new resource to MCP server
- OnFileChanged → Update existing resource in MCP server
- OnFileDeleted → Remove resource from MCP server
- OnFileRenamed → Remove old resource, add new resource
- Client receives resource change notification automatically

**SDK Pattern**: Follow ModelContextProtocol 0.4.0 resource update API

---

### FR-5: Cleanup Obsolete SDK 0.3.0 References
**Priority**: MUST HAVE
**Description**: Remove deprecated SDK patterns and obsolete tool references

**Cleanup Items**:
1. `src/assets/workflows/workflow-dispatch.json:36` - Remove `ListMcpResourcesTool` reference (obsolete)
2. `src/assets/colors/blue.json:39` - Remove `resource://` URI references (SDK 0.3.0 pattern)

---

## Non-Functional Requirements

### NFR-1: Performance
**Requirement**: Asset hot-loading must not degrade system performance

**Metrics**:
- Asset reload latency: <500ms from file save to MCP client notification
- Memory sync operations: no performance degradation
- Build time: no significant increase (±5% acceptable)

---

### NFR-2: Reliability
**Requirement**: All existing functionality must remain stable

**Metrics**:
- All 161 existing tests must pass on SDK 0.4.0
- New integration tests: minimum 3 tests for asset hot-loading
  - Test 1: File creation triggers resource appearance
  - Test 2: File modification triggers resource update
  - Test 3: File deletion triggers resource removal
- Test implementation: Real FileSystemWatcher, real files (NO MOCKS per Ma Protocol)

**Test Files**:
- tests/Maenifold.Tests/IncrementalSyncToolsTests.cs (reference pattern)
- tests/Maenifold.Tests/AssetHotLoadingTests.cs (new file, 3+ tests)

---

### NFR-3: Compatibility
**Requirement**: Clean break from SDK 0.3.0, no backward compatibility

**Constraints**:
- Breaking changes from SDK 0.3.0 to 0.4.0 are ACCEPTABLE
- No legacy code paths for SDK 0.3.0 support
- Remove obsolete patterns immediately (no deprecation warnings)
- RTM explicitly states: "No backward compatibility with 0.3.x"

---

### NFR-4: Architecture
**Requirement**: Follow Ma Protocol and SDK 0.4.0 patterns

**Principles**:
- NO MOCKS in tests (Ma Protocol: NO FAKE TESTS)
- NO FAKE AI - errors propagate to LLM with complete information
- NO UNNECESSARY ABSTRACTIONS - direct SDK usage, no wrapper layers
- NO FAKE SECURITY - prepared statements only, trust user's file system
- Reuse existing patterns: IncrementalSyncTools FileSystemWatcher implementation
- Follow SDK 0.4.0 attribute-based resource pattern exactly

---

## Acceptance Criteria

### AC-1: SDK Upgrade Complete
- [ ] Maenifold.csproj updated to ModelContextProtocol 0.4.0-preview.3
- [ ] All MCP server initialization code compiles without errors
- [ ] Server starts successfully in `--mcp` mode
- [ ] All existing MCP tools remain functional (40 tools tested)

### AC-2: Asset Hot-Loading Functional
- [ ] AssetManager exposes workflow definitions via MCP resources
- [ ] AssetManager exposes role definitions via MCP resources
- [ ] AssetManager exposes color definitions via MCP resources
- [ ] AssetManager exposes perspective definitions via MCP resources
- [ ] FileSystemWatcher monitors ~/maenifold/assets/**/*.json
- [ ] Changes to asset files trigger resource update notifications
- [ ] LLM can access updated assets without server restart

### AC-3: Testing Complete
- [ ] All 161 existing tests pass on new SDK version
- [ ] 3+ new integration tests for asset hot-loading pass
- [ ] Tests use real FileSystemWatcher, real files (Ma Protocol compliant)
- [ ] No mocks, no stubs in test implementation

### AC-4: Build Success
- [ ] `dotnet build` succeeds with zero errors
- [ ] `dotnet test` passes all tests
- [ ] `dotnet publish` generates working binaries
- [ ] No new compiler warnings introduced

### AC-5: Cleanup Items
- [ ] workflow-dispatch.json:36 updated (remove ListMcpResourcesTool reference)
- [ ] blue.json:39 updated (remove resource:// URI references)
- [ ] No obsolete SDK 0.3.0 patterns remain in codebase

---

## Implementation Constraints

### Ma Protocol Compliance
- NO MOCKS in tests - use real FileSystemWatcher, real files
- NO FAKE AI - errors propagate to LLM with complete information
- NO UNNECESSARY ABSTRACTIONS - direct SDK usage, no wrapper layers
- NO FAKE SECURITY - prepared statements only, trust user's file system

### SDK Migration Path
- Breaking changes from 0.3.0 to 0.4.0 are ACCEPTABLE
- No backward compatibility required
- Clean break - remove obsolete patterns immediately
- Follow SDK 0.4.0 attribute-based resource pattern exactly

### FileSystemWatcher Pattern
- Reuse IncrementalSyncTools.cs implementation pattern
- Debouncing for rapid file changes (150ms default)
- Error handling via OnWatcherError event
- Buffer size: Config.WatcherBufferSize (64KB)

---

## Risks and Mitigation

### RISK-1: SDK API Breaking Changes (HIGH)
**Impact**: Server won't start if builder chain API changed
**Mitigation**: Read SDK 0.4.0 changelog/docs before modifying Program.cs
**Detection**: Compiler errors will surface immediately

### RISK-2: Attribute Pattern Unknown (MEDIUM)
**Impact**: Resource registration may require new attributes/methods
**Mitigation**: Research SDK 0.4.0 examples for [McpServerResource] usage
**Detection**: Resources won't appear in MCP client if wrong pattern

### RISK-3: FileSystemWatcher Instability (LOW)
**Impact**: Asset changes might not trigger notifications on all platforms
**Mitigation**: Test on macOS (dev platform) with real file operations
**Detection**: Integration tests will fail if watcher doesn't fire

### RISK-4: Test Failures on Migration (MEDIUM)
**Impact**: Existing tests may break if SDK changed test infrastructure
**Mitigation**: Run `dotnet test` immediately after SDK upgrade
**Detection**: CI will fail, manual verification required

---

## Dependencies

- **ModelContextProtocol 0.4.0-preview.3** (NuGet package)
- **serana/codenav** for C# analysis (already available)
- **Real file system** for testing (Ma Protocol requirement)

---

## References

- **RTM**: RTM.md
- **Discovery Analysis**: docs/mcp-sdk-upgrade-analysis.md
- **Sequential Thinking Session**: memory://thinking/sequential/session-1762118296713.md
- **Reference Implementation**: src/Tools/IncrementalSyncTools.cs
- **Reference Tests**: tests/Maenifold.Tests/IncrementalSyncToolsTests.cs
