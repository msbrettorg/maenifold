# Technical Specification: FR-1.x Core Plugin Infrastructure

## Document Control

| Version | Date | Author | RTM Reference |
|---------|------|--------|---------------|
| 1.0 | 2026-01-28 | SWE Agent | T-SPEC-1, FR-1.1–FR-1.4 |

---

## 1. Overview

This specification defines the core infrastructure for the [[OpenCode]] plugin that integrates [[maenifold]]'s knowledge graph. It covers:

- **FR-1.1**: Plugin export structure (OpenCode plugin API compliance)
- **FR-1.2**: CLI binary discovery (PATH and MAENIFOLD_ROOT)
- **FR-1.3**: Graceful degradation when CLI is unavailable
- **FR-1.4**: CLI command execution with configurable timeout

---

## 2. Function Signatures

### 2.1 Plugin Entry Point (FR-1.1)

```typescript
// T-SPEC-1.1: RTM FR-1.1
import type { Plugin } from "@opencode-ai/plugin";

/**
 * Default export compatible with OpenCode plugin API.
 * Receives context object with project, client, $, directory, worktree.
 * Returns object with event handlers and optional custom tools.
 */
export default (async ({ project, client, $, directory, worktree }) => {
  // Initialize CLI path discovery
  const cliPath = findMaenifoldCli();
  
  // Return hook handlers (FR-2.x, FR-3.x, FR-4.x defined in separate specs)
  return {
    event: async ({ event }) => { /* FR-2.x */ },
    "experimental.session.compacting": async (input, output) => { /* FR-3.x */ },
    "tool.execute.before": async (input, output) => { /* FR-4.x */ },
  };
}) satisfies Plugin;
```

**Type Definition** (from `@opencode-ai/plugin`):

```typescript
type PluginContext = {
  project: {
    id: string;
    worktree: string;
    vcsDir?: string;
    vcs?: "git";
    time: { created: number; initialized?: number };
  };
  client: OpencodeClient;  // SDK client for API calls
  $: BunShell;             // Bun shell for command execution
  directory: string;       // Current working directory
  worktree: string;        // Git worktree path
};

type Plugin = (ctx: PluginContext) => Promise<PluginHandlers>;
```

### 2.2 CLI Discovery (FR-1.2)

```typescript
// T-SPEC-1.2: RTM FR-1.2
/**
 * Locate maenifold CLI binary via PATH or MAENIFOLD_ROOT env var.
 * 
 * @returns Absolute path to maenifold binary, or null if not found
 * @security Validates resolved path against symlink traversal attacks
 */
function findMaenifoldCli(): string | null;
```

**Implementation Contract**:

```typescript
interface CliDiscoveryResult {
  path: string | null;
  source: "MAENIFOLD_ROOT" | "PATH" | null;
  error?: string;
}

/**
 * Extended discovery function for debugging/logging purposes.
 * The simple findMaenifoldCli() wraps this internally.
 */
function discoverMaenifoldCli(): CliDiscoveryResult;
```

### 2.3 Graceful Degradation (FR-1.3)

```typescript
// T-SPEC-1.3: RTM FR-1.3
/**
 * State object tracking CLI availability.
 * Used by all hook handlers to determine behavior.
 */
interface PluginState {
  cliAvailable: boolean;
  cliPath: string | null;
  initError?: string;
}

/**
 * Initialize plugin state. Called once at plugin load.
 * Sets cliAvailable=false if CLI not found (graceful degradation).
 */
function initializePluginState(): PluginState;
```

### 2.4 CLI Execution (FR-1.4)

```typescript
// T-SPEC-1.4: RTM FR-1.4
/**
 * Execute a maenifold CLI tool with JSON payload.
 * 
 * @param tool - Tool name (e.g., "SearchMemories", "BuildContext")
 * @param payload - JSON-serializable payload object
 * @param options - Execution options including timeout
 * @returns Parsed JSON response or empty object on failure
 * @throws Never throws - returns empty object on any error
 */
async function runMaenifoldTool<T = unknown>(
  tool: string,
  payload: object,
  options?: ExecutionOptions
): Promise<T>;

interface ExecutionOptions {
  /** Timeout in milliseconds. Default: 5000 (5s) */
  timeout?: number;
  /** Working directory for command execution */
  cwd?: string;
}

/**
 * Lower-level execution returning raw output and metadata.
 * Used internally by runMaenifoldTool.
 */
async function executeCliCommand(
  args: string[],
  options?: ExecutionOptions
): Promise<ExecutionResult>;

interface ExecutionResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exitCode: number | null;
  timedOut: boolean;
  duration: number;
}
```

---

## 3. Parameters and Return Types

### 3.1 FR-1.1 Plugin Export

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `ctx.project` | `Project` | Yes | Current OpenCode project info |
| `ctx.client` | `OpencodeClient` | Yes | SDK client for API calls |
| `ctx.$` | `BunShell` | Yes | Bun shell for commands |
| `ctx.directory` | `string` | Yes | Current working directory |
| `ctx.worktree` | `string` | Yes | Git worktree path |

**Return**: `Promise<PluginHandlers>` with event hook functions.

### 3.2 FR-1.2 CLI Discovery

| Return Field | Type | Description |
|--------------|------|-------------|
| `path` | `string \| null` | Absolute resolved path to CLI binary |
| `source` | `"MAENIFOLD_ROOT" \| "PATH" \| null` | Discovery method used |
| `error` | `string \| undefined` | Error message if discovery failed |

**Environment Variables**:

| Variable | Description | Example |
|----------|-------------|---------|
| `MAENIFOLD_ROOT` | Root directory containing maenifold binary | `/usr/local/maenifold` |
| `PATH` | System PATH for binary lookup | Standard system PATH |

### 3.3 FR-1.3 Plugin State

| Field | Type | Description |
|-------|------|-------------|
| `cliAvailable` | `boolean` | True if CLI was found and validated |
| `cliPath` | `string \| null` | Path to CLI if available |
| `initError` | `string \| undefined` | Error message from initialization |

### 3.4 FR-1.4 CLI Execution

**ExecutionOptions**:

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout` | `number` | `5000` | Command timeout in milliseconds |
| `cwd` | `string` | `undefined` | Working directory for execution |

**ExecutionResult**:

| Field | Type | Description |
|-------|------|-------------|
| `success` | `boolean` | True if exitCode === 0 and no timeout |
| `stdout` | `string` | Standard output from command |
| `stderr` | `string` | Standard error from command |
| `exitCode` | `number \| null` | Exit code (null if killed/timeout) |
| `timedOut` | `boolean` | True if command exceeded timeout |
| `duration` | `number` | Execution time in milliseconds |

---

## 4. Error Handling Approach

### 4.1 Design Principles

Following [[Ma Protocol]] philosophy:
1. **Surface errors, don't hide them**: Log errors via `client.app.log()` but don't throw
2. **Graceful degradation over failure**: Return empty/neutral values when CLI unavailable
3. **No retry logic**: Let errors surface; user can retry manually
4. **Structured error context**: Include tool name, payload summary, and timing

### 4.2 FR-1.1 Plugin Initialization Errors

```typescript
// T-SPEC-1.1: RTM FR-1.1 - Error handling
try {
  const state = initializePluginState();
  if (!state.cliAvailable) {
    await client.app.log({
      body: {
        service: "maenifold-plugin",
        level: "warn",
        message: "maenifold CLI not found, plugin operating in degraded mode",
        extra: { error: state.initError }
      }
    });
  }
} catch (e) {
  // Plugin initialization must not throw - log and continue
  console.error("[maenifold-plugin] Initialization failed:", e);
}
```

### 4.3 FR-1.2 Discovery Errors

| Condition | Behavior | Return |
|-----------|----------|--------|
| MAENIFOLD_ROOT set but no binary | Log warning | `{ path: null, source: null, error: "Binary not found at MAENIFOLD_ROOT" }` |
| PATH search finds no binary | Silent (expected) | `{ path: null, source: null }` |
| Symlink resolves outside allowed paths | Log warning | `{ path: null, source: null, error: "Path traversal detected" }` |
| Binary found but not executable | Log warning | `{ path: null, source: null, error: "Binary not executable" }` |

### 4.4 FR-1.3 Degradation Behavior

When `cliAvailable === false`, all hook handlers:
1. Return immediately with empty/neutral values
2. Do not log on every invocation (avoid log spam)
3. Allow OpenCode to function normally without graph features

```typescript
// Example degradation pattern
if (!state.cliAvailable) {
  return; // No-op, let OpenCode proceed normally
}
```

### 4.5 FR-1.4 Execution Errors

| Error Type | Behavior | Return |
|------------|----------|--------|
| Timeout (>5s) | Kill process, log warning | `{}` (empty object) |
| Non-zero exit | Log error with stderr | `{}` (empty object) |
| JSON parse failure | Log error with raw output | `{}` (empty object) |
| Process spawn failure | Log error | `{}` (empty object) |

```typescript
// T-SPEC-1.4: RTM FR-1.4 - Error handling pattern
async function runMaenifoldTool<T>(tool: string, payload: object, options?: ExecutionOptions): Promise<T> {
  const result = await executeCliCommand(
    ["--tool", tool, "--payload", JSON.stringify(payload)],
    options
  );
  
  if (result.timedOut) {
    await logError("CLI command timed out", { tool, duration: result.duration });
    return {} as T;
  }
  
  if (!result.success) {
    await logError("CLI command failed", { tool, exitCode: result.exitCode, stderr: result.stderr });
    return {} as T;
  }
  
  try {
    return JSON.parse(result.stdout) as T;
  } catch (e) {
    await logError("Failed to parse CLI output", { tool, output: result.stdout.slice(0, 500) });
    return {} as T;
  }
}
```

---

## 5. Security Considerations

### 5.1 Path Traversal Prevention (FR-1.2)

**Threat**: Symlink attack via MAENIFOLD_ROOT pointing to malicious binary.

**Mitigation**:

```typescript
// T-SPEC-1.2: RTM FR-1.2 - Security
import { realpathSync, existsSync, accessSync, constants } from "fs";
import { resolve, dirname } from "path";

function validateCliPath(candidatePath: string): string | null {
  // 1. Resolve symlinks to get true path
  let resolvedPath: string;
  try {
    resolvedPath = realpathSync(candidatePath);
  } catch {
    return null; // Path doesn't exist or can't be resolved
  }
  
  // 2. If MAENIFOLD_ROOT is set, verify resolved path is within it
  const root = process.env.MAENIFOLD_ROOT;
  if (root) {
    const resolvedRoot = realpathSync(root);
    // CRITICAL: Append path separator to prevent sibling bypass
    // e.g., /safe-evil vs /safe
    if (!resolvedPath.startsWith(resolvedRoot + "/") && resolvedPath !== resolvedRoot) {
      return null; // Path traversal attempt
    }
  }
  
  // 3. Verify binary is executable
  try {
    accessSync(resolvedPath, constants.X_OK);
  } catch {
    return null; // Not executable
  }
  
  return resolvedPath;
}
```

**Validation Rules**:
1. Always resolve symlinks with `realpathSync()` before path validation
2. When MAENIFOLD_ROOT is set, verify resolved path starts with `resolvedRoot + "/"` (note trailing separator)
3. Verify binary has executable permission
4. Log security-relevant failures with sufficient context

### 5.2 Command Injection Prevention (FR-1.4)

**Threat**: Malicious payload content escaping JSON and executing arbitrary commands.

**Mitigation**:

```typescript
// T-SPEC-1.4: RTM FR-1.4 - Security
async function executeCliCommand(args: string[], options?: ExecutionOptions): Promise<ExecutionResult> {
  // Use Bun.spawn with explicit argument array - NOT shell interpolation
  // This prevents command injection via payload content
  const proc = Bun.spawn([state.cliPath!, ...args], {
    cwd: options?.cwd,
    stdout: "pipe",
    stderr: "pipe",
  });
  
  // ... timeout and result handling
}
```

**Key Security Properties**:
1. **Never** use template literals or string concatenation with shell execution
2. **Always** pass arguments as array to `Bun.spawn()`
3. Payload is passed as single `--payload` argument value (JSON string)
4. CLI binary handles JSON parsing; plugin does not interpret payload content

### 5.3 JSON Parsing Safety (FR-1.4)

**Threat**: Malformed JSON response causing unexpected behavior or DoS.

**Mitigation**:

```typescript
// Safe JSON parsing with depth limit (aligns with maenifold's SafeJson pattern)
function safeJsonParse<T>(text: string, maxLength: number = 1_000_000): T | null {
  if (text.length > maxLength) {
    return null; // Prevent DoS via huge response
  }
  try {
    return JSON.parse(text) as T;
  } catch {
    return null;
  }
}
```

### 5.4 Environment Variable Validation (FR-1.2)

**Threat**: MAENIFOLD_ROOT containing special characters or path components that could cause unexpected behavior.

**Mitigation**:

```typescript
function sanitizeEnvPath(envValue: string | undefined): string | null {
  if (!envValue) return null;
  
  // Reject obviously malicious patterns
  if (envValue.includes('\0')) return null;  // Null byte injection
  if (envValue.includes('..')) return null;  // Path traversal attempt
  
  // Normalize path (resolve . and .. components)
  const normalized = resolve(envValue);
  
  // Verify it's an absolute path
  if (!normalized.startsWith('/')) return null;
  
  return normalized;
}
```

### 5.5 Timeout Handling Security (FR-1.4)

**Threat**: CLI process hanging indefinitely, consuming resources.

**Mitigation**:

```typescript
async function executeWithTimeout(
  proc: Subprocess,
  timeout: number
): Promise<{ output: string; timedOut: boolean }> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(() => reject(new Error("timeout")), timeout);
  });
  
  try {
    const output = await Promise.race([
      new Response(proc.stdout).text(),
      timeoutPromise
    ]);
    return { output, timedOut: false };
  } catch {
    // Kill the process on timeout
    proc.kill("SIGTERM");
    
    // Give it 1 second to terminate gracefully
    await new Promise(r => setTimeout(r, 1000));
    
    // Force kill if still running
    try {
      proc.kill("SIGKILL");
    } catch {
      // Process may have already exited
    }
    
    return { output: "", timedOut: true };
  }
}
```

---

## 6. Configuration

### 6.1 Environment Variables

| Variable | Type | Default | Description |
|----------|------|---------|-------------|
| `MAENIFOLD_ROOT` | `string` | (none) | Root directory containing maenifold binary |
| `MAENIFOLD_TIMEOUT` | `number` | `5000` | CLI command timeout in milliseconds |
| `MAENIFOLD_DEBUG` | `boolean` | `false` | Enable verbose logging |

### 6.2 Configuration Loading

```typescript
// T-SPEC-1.4: RTM FR-1.4 - Configuration
interface PluginConfig {
  timeout: number;
  debug: boolean;
}

function loadConfig(): PluginConfig {
  return {
    timeout: parseEnvInt("MAENIFOLD_TIMEOUT", 5000, { min: 1000, max: 30000 }),
    debug: process.env.MAENIFOLD_DEBUG === "true",
  };
}

function parseEnvInt(
  name: string,
  defaultValue: number,
  constraints?: { min?: number; max?: number }
): number {
  const raw = process.env[name];
  if (!raw) return defaultValue;
  
  const parsed = parseInt(raw, 10);
  if (isNaN(parsed)) return defaultValue;
  
  if (constraints?.min !== undefined && parsed < constraints.min) return constraints.min;
  if (constraints?.max !== undefined && parsed > constraints.max) return constraints.max;
  
  return parsed;
}
```

---

## 7. Implementation Notes

### 7.1 Bun Runtime Considerations

The plugin runs in [[Bun]] runtime (OpenCode's environment):
- Use `Bun.spawn()` for subprocess execution (not Node's `child_process`)
- Use `Bun.file()` for file system operations if needed
- Shell API (`$`) is available but avoid for CLI execution (security)

### 7.2 Async/Await Patterns

All hook handlers and CLI execution are async:
- Never block the event loop with synchronous operations
- Use proper timeout handling with `Promise.race()`
- Clean up resources (kill processes) on timeout

### 7.3 Logging Best Practices

```typescript
// Structured logging via OpenCode SDK
await client.app.log({
  body: {
    service: "maenifold-plugin",
    level: "info",  // debug, info, warn, error
    message: "Descriptive message",
    extra: { /* structured context */ }
  }
});
```

- Use `debug` level for verbose operation tracing
- Use `info` for significant state changes
- Use `warn` for degradation scenarios
- Use `error` for failures that affect functionality

---

## 8. Test Strategy

### 8.1 Unit Test Coverage

| Function | Test Cases |
|----------|------------|
| `findMaenifoldCli()` | PATH lookup, MAENIFOLD_ROOT lookup, symlink resolution, missing binary |
| `validateCliPath()` | Path traversal, sibling bypass, symlink attack, non-executable |
| `runMaenifoldTool()` | Success, timeout, parse error, exit code non-zero |
| `parseEnvInt()` | Valid, invalid, out of range, missing |

### 8.2 Integration Test Scenarios

| Scenario | Expected Behavior |
|----------|-------------------|
| CLI not installed | Plugin loads, logs warning, operates in degraded mode |
| CLI installed in PATH | Plugin discovers and uses CLI normally |
| CLI in MAENIFOLD_ROOT | Plugin prefers MAENIFOLD_ROOT over PATH |
| CLI times out | Returns empty object, logs warning |
| Symlink outside root | Rejects path, logs security warning |

---

## 9. References

- [[OpenCode]] Plugin API Reference: `memory://tech/integrations/opencode-plugin-api-reference`
- [[maenifold]] CLI Scripting: `docs/SCRIPTING.md`
- [[Ma Protocol]] Philosophy: `docs/MA_MANIFESTO.md`
- Security Remediation (T-OC-6): `memory://tech/integrations/opencode-plugin-for-maenifold`
