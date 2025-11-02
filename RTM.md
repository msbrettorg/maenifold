# Requirements Traceability Matrix - MCP SDK Upgrade Sprint

**Sprint ID**: `sprint-20251102-mcp-sdk-upgrade`
**Branch**: `sprint-20251102-mcp-sdk-upgrade`
**Workflow Session**: `workflow-1762117950582`
**Specifications Session**: `session-1762118296713`
**RTM Session**: `session-1762118508005`

---

## MUST HAVE: Atomic Functionality Units

### MCP SDK Upgrade (FR-1)

- [ ] **RTM-001**: Maenifold.csproj PackageReference updated to ModelContextProtocol 0.4.0-preview.3
  - **Spec**: SPRINT_SPECIFICATIONS.md:17
  - **Test**: Verify src/Maenifold.csproj:35 contains Version="0.4.0-preview.3"

- [ ] **RTM-002**: MCP server builder chain compiles without errors
  - **Spec**: SPRINT_SPECIFICATIONS.md:18
  - **Test**: `dotnet build` returns exit code 0

- [ ] **RTM-003**: MCP server starts in --mcp mode without exceptions
  - **Spec**: SPRINT_SPECIFICATIONS.md:18
  - **Test**: Execute `maenifold --mcp` and verify no startup exceptions

- [ ] **RTM-004**: All 40 existing MCP tools remain functional
  - **Spec**: SPRINT_SPECIFICATIONS.md:19
  - **Test**: Tool discovery count equals 40 in MCP server

### Asset Hot-Loading via MCP Resources (FR-2)

- [ ] **RTM-005**: AssetManager implements [McpServerResource] attribute pattern
  - **Spec**: SPRINT_SPECIFICATIONS.md:34
  - **Test**: AssetManager class has resource methods with [McpServerResource] attributes

- [ ] **RTM-006**: AssetManager.GetWorkflowResources() returns 28 workflow JSON resources
  - **Spec**: SPRINT_SPECIFICATIONS.md:35
  - **Test**: Resource count from workflows directory equals 28

- [ ] **RTM-007**: AssetManager.GetRoleResources() returns 7 role JSON resources
  - **Spec**: SPRINT_SPECIFICATIONS.md:36
  - **Test**: Resource count from roles directory equals 7

- [ ] **RTM-008**: AssetManager.GetColorResources() returns 7 color JSON resources
  - **Spec**: SPRINT_SPECIFICATIONS.md:37
  - **Test**: Resource count from colors directory equals 7

- [ ] **RTM-009**: AssetManager.GetPerspectiveResources() returns 12 perspective JSON resources
  - **Spec**: SPRINT_SPECIFICATIONS.md:38
  - **Test**: Resource count from perspectives directory equals 12

- [ ] **RTM-010**: Total MCP resource count equals 54 (workflows + roles + colors + perspectives)
  - **Spec**: SPRINT_SPECIFICATIONS.md:39
  - **Test**: Sum of all asset resources equals 54

### FileSystemWatcher for Asset Changes (FR-3)

- [ ] **RTM-011**: AssetWatcher monitors ~/maenifold/assets/ with *.json filter
  - **Spec**: SPRINT_SPECIFICATIONS.md:55-56
  - **Test**: FileSystemWatcher.Path equals assets directory AND Filter equals "*.json"

- [ ] **RTM-012**: AssetWatcher IncludeSubdirectories is true
  - **Spec**: SPRINT_SPECIFICATIONS.md:56
  - **Test**: FileSystemWatcher.IncludeSubdirectories property is true

- [ ] **RTM-013**: AssetWatcher NotifyFilter includes FileName and LastWrite
  - **Spec**: SPRINT_SPECIFICATIONS.md:58
  - **Test**: NotifyFilter has FileName | LastWrite flags

- [ ] **RTM-014**: AssetWatcher uses 150ms debounce delay
  - **Spec**: SPRINT_SPECIFICATIONS.md:57
  - **Test**: Debounce timer interval equals Config.DefaultDebounceMs (150ms)

- [ ] **RTM-015**: AssetWatcher handles Created, Changed, Deleted, Renamed, Error events
  - **Spec**: SPRINT_SPECIFICATIONS.md:60
  - **Test**: Event handlers registered for all 5 event types

- [ ] **RTM-016**: AssetWatcher InternalBufferSize equals 64KB
  - **Spec**: SPRINT_SPECIFICATIONS.md:59
  - **Test**: FileSystemWatcher.InternalBufferSize equals Config.WatcherBufferSize

### Resource Update Notifications (FR-4)

- [ ] **RTM-017**: OnFileCreated adds new resource to MCP server
  - **Spec**: SPRINT_SPECIFICATIONS.md:71
  - **Test**: Create test.json in assets/, verify resource appears in MCP resource list

- [ ] **RTM-018**: OnFileChanged updates existing resource in MCP server
  - **Spec**: SPRINT_SPECIFICATIONS.md:72
  - **Test**: Modify existing asset JSON, verify resource content updates

- [ ] **RTM-019**: OnFileDeleted removes resource from MCP server
  - **Spec**: SPRINT_SPECIFICATIONS.md:73
  - **Test**: Delete asset JSON, verify resource disappears from MCP resource list

- [ ] **RTM-020**: OnFileRenamed removes old resource and adds new resource
  - **Spec**: SPRINT_SPECIFICATIONS.md:74
  - **Test**: Rename asset JSON, verify old resource removed and new resource added

### Cleanup Obsolete SDK 0.3.0 References (FR-5)

- [ ] **RTM-021**: workflow-dispatch.json:36 no longer references ListMcpResourcesTool
  - **Spec**: SPRINT_SPECIFICATIONS.md:86
  - **Test**: Grep workflow-dispatch.json for "ListMcpResourcesTool" returns no matches

- [ ] **RTM-022**: blue.json:39 no longer contains resource:// URI references
  - **Spec**: SPRINT_SPECIFICATIONS.md:87
  - **Test**: Grep blue.json for "resource://" returns no matches

### Non-Functional Requirements

#### Performance (NFR-1)

- [ ] **RTM-023**: Asset reload latency under 500ms
  - **Spec**: SPRINT_SPECIFICATIONS.md:97
  - **Test**: Measure time from file save to MCP client notification, assert < 500ms

#### Reliability (NFR-2)

- [ ] **RTM-024**: All 161 existing tests pass on SDK 0.4.0
  - **Spec**: SPRINT_SPECIFICATIONS.md:107
  - **Test**: `dotnet test` shows 161 passed, 0 failed

- [ ] **RTM-025**: AssetHotLoadingTests has 3+ integration tests
  - **Spec**: SPRINT_SPECIFICATIONS.md:108-111
  - **Test**: Count test methods in AssetHotLoadingTests.cs >= 3

- [ ] **RTM-026**: AssetHotLoadingTests uses real FileSystemWatcher (no mocks)
  - **Spec**: SPRINT_SPECIFICATIONS.md:112
  - **Test**: Grep AssetHotLoadingTests.cs for "Mock" returns no matches (Ma Protocol compliance)

#### Build Success (AC-4)

- [ ] **RTM-027**: `dotnet build` succeeds with zero errors
  - **Spec**: SPRINT_SPECIFICATIONS.md:168
  - **Test**: Build exit code 0 AND error count 0

- [ ] **RTM-028**: `dotnet test` passes all tests
  - **Spec**: SPRINT_SPECIFICATIONS.md:169
  - **Test**: Test exit code 0 AND no test failures

- [ ] **RTM-029**: `dotnet publish` generates working binaries
  - **Spec**: SPRINT_SPECIFICATIONS.md:170
  - **Test**: Publish succeeds AND binary executes `--version` successfully

- [ ] **RTM-030**: No new compiler warnings introduced
  - **Spec**: SPRINT_SPECIFICATIONS.md:171
  - **Test**: Warning count before upgrade equals warning count after upgrade

---

## MUST NOT HAVE: Constraints

- [ ] **RTM-X01**: No backward compatibility with SDK 0.3.x
  - **Spec**: SPRINT_SPECIFICATIONS.md:124-125
  - **Verification**: No conditional code paths for SDK 0.3.0 support

- [ ] **RTM-X02**: No scope creep beyond SDK upgrade and asset hot-loading
  - **Spec**: SPRINT_SPECIFICATIONS.md (entire document scope)
  - **Verification**: Git diff only touches SDK version, AssetManager, FileSystemWatcher, tests, cleanup items

- [ ] **RTM-X03**: No mocks in tests (Ma Protocol compliance)
  - **Spec**: SPRINT_SPECIFICATIONS.md:135, 183
  - **Verification**: Grep test files for "Mock", "Stub", "Fake" patterns returns zero matches

- [ ] **RTM-X04**: No unnecessary abstractions (Ma Protocol compliance)
  - **Spec**: SPRINT_SPECIFICATIONS.md:136, 185
  - **Verification**: No new interfaces, no new factory patterns, direct SDK usage

---

## Traceability Summary

**MUST HAVE**: 30 atomic functionality units
- SDK Upgrade: 4 items (RTM-001 to RTM-004)
- Asset Hot-Loading: 6 items (RTM-005 to RTM-010)
- FileSystemWatcher: 6 items (RTM-011 to RTM-016)
- Resource Notifications: 4 items (RTM-017 to RTM-020)
- Cleanup: 2 items (RTM-021 to RTM-022)
- Non-Functional: 8 items (RTM-023 to RTM-030)

**MUST NOT HAVE**: 4 constraint items
- RTM-X01: No backward compatibility
- RTM-X02: No scope creep
- RTM-X03: No mocks
- RTM-X04: No unnecessary abstractions

**Total RTM Items**: 34 (30 MUST HAVE + 4 MUST NOT HAVE)

---

## Atomicity Verification

✅ Every RTM item is ONE testable behavior
✅ Every RTM item traces to specific specification line
✅ Every RTM item has clear pass/fail criteria
✅ No RTM item requires further decomposition

**ESCAPE HATCHES TRIGGERED**: None
- All requirements decomposed to atomic level
- Git diff scope matches RTM scope
- No non-atomic items detected

---

## References

- **Specifications**: SPRINT_SPECIFICATIONS.md
- **Discovery Analysis**: docs/mcp-sdk-upgrade-analysis.md
- **RTM Sequential Thinking**: memory://thinking/sequential/session-1762118508005.md
- **Specifications Thinking**: memory://thinking/sequential/session-1762118296713.md
