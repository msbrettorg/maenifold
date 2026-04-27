# Maenifold Skill for Claude Code

This skill provides Claude Code with access to maenifold's knowledge graph and reasoning infrastructure. It enables persistent memory, graph-based context building, structured workflows, and multi-session thinking continuity.

## Overview

The maenifold skill exposes 25+ MCP tools that transform ephemeral AI sessions into continuous collective intelligence:

- **Memory Tools**: Write, read, search, edit, and organize knowledge files with `[[WikiLink]]` graph integration
- **Graph Tools**: Build context, find similar concepts, visualize relationships, and sync the knowledge graph
- **Reasoning Tools**: Sequential thinking with branching, structured workflows, and assumption tracking
- **System Tools**: Adopt roles/perspectives, monitor activity, and manage configuration

Every `[[WikiLink]]` you create becomes a node in the graph. Knowledge compounds across sessions instead of resetting.

## Cognitive Architecture

Maenifold operates as a **6-layer composition architecture** where higher layers invoke lower layers. Complexity emerges from composition, not bloated tools.

### 6-Layer Stack

_6-Layer Stack — diagram_ ::
Nodes: Workflow / Multi-step processes / Nested workflows · Sequential Thinking / Branching, revision / Multi-session persistence · Adopt / Roles, colors, perspectives / Conditioned reasoning · Recent Activity / Assumption Ledger / State tracking · Write/Read/Search/Edit / BuildContext, FindSimilar / Persist & Query · WikiLinks / Atomic units / Graph nodes.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph TB
    subgraph "Layer 6: Orchestration"
        Orchestration[Workflow<br/>Multi-step processes<br/>Nested workflows]
    end

    subgraph "Layer 5: Reasoning"
        Reasoning[Sequential Thinking<br/>Branching, revision<br/>Multi-session persistence]
    end

    subgraph "Layer 4: Persona"
        Persona[Adopt<br/>Roles, colors, perspectives<br/>Conditioned reasoning]
    end

    subgraph "Layer 3: Session"
        Session[Recent Activity<br/>Assumption Ledger<br/>State tracking]
    end

    subgraph "Layer 2: Memory + Graph"
        Memory[Write/Read/Search/Edit<br/>BuildContext, FindSimilar<br/>Persist & Query]
    end

    subgraph "Layer 1: Concepts"
        Concepts["WikiLinks<br/>Atomic units<br/>Graph nodes"]
    end

    Orchestration -->|invokes| Reasoning
    Reasoning -->|conditions| Persona
    Persona -->|tracks| Session
    Session -->|queries| Memory
    Memory -->|built from| Concepts

    style Orchestration fill:#0969DA
    style Reasoning fill:#0969DA
    style Persona fill:#0969DA
    style Session fill:#0969DA
    style Memory fill:#0969DA
    style Concepts fill:#0969DA
```

</details>

**Layer 1: Concepts** - Every `[[WikiLink]]` becomes a graph node. Foundation for all knowledge.

**Layer 2: Memory + Graph** - Tools persist (`writememory`) and query (`searchmemories`, `buildcontext`, `findsimilarconcepts`) knowledge.

**Layer 3: Session** - Track state across interactions (`recentactivity`, `assumptionledger`).

**Layer 4: Persona** - Condition reasoning through roles/colors/perspectives (`adopt`).

**Layer 5: Reasoning** - Enable revision, branching, multi-day persistence (`sequentialthinking`).

**Layer 6: Orchestration** - Compose all layers; workflows can nest workflows (`workflow`).

### Tool Composition Patterns

#### Chain Pattern

_Chain Pattern — diagram_ ::
Nodes: findsimilarconcepts / Discover variants · Pick best match · buildcontext / Get relationships · searchmemories / Find files · readmemory / Access content.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph LR
    A[findsimilarconcepts<br/>Discover variants] --> B[Pick best match]
    B --> C[buildcontext<br/>Get relationships]
    C --> D[searchmemories<br/>Find files]
    D --> E[readmemory<br/>Access content]

    style A fill:#0969DA
    style B fill:#0969DA
    style C fill:#0969DA
    style D fill:#0969DA
    style E fill:#0969DA
```

</details>

**Use case**: Navigate from unknown domain to specific knowledge.

**Example**:
1. `findsimilarconcepts("authentication")` → finds [[auth]], [[authentication]], [[JWT]]
2. Pick [[authentication]] as best match
3. `buildcontext("authentication", depth=2)` → related concepts + files
4. `searchmemories("JWT authentication patterns")` → relevant memory files
5. `readmemory("memory://security/jwt-implementation")` → specific content

#### HYDE Pattern (Hypothetical Document Embeddings)

_HYDE Pattern (Hypothetical Document Embeddings) — diagram_ ::
Nodes: Synthesize hypothetical answer / with concepts inline · Extract concepts / from synthesis · buildcontext for / each concept · searchmemories using / enriched context · readmemory for / relevant files · Answer with / actual knowledge.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph TD
    A[Synthesize hypothetical answer<br/>with concepts inline] --> B[Extract concepts<br/>from synthesis]
    B --> C[buildcontext for<br/>each concept]
    C --> D[searchmemories using<br/>enriched context]
    D --> E[readmemory for<br/>relevant files]
    E --> F[Answer with<br/>actual knowledge]

    style A fill:#0969DA
    style B fill:#0969DA
    style C fill:#0969DA
    style D fill:#0969DA
    style E fill:#0969DA
    style F fill:#0969DA
```

</details>

**Use case**: Answer complex questions by first hypothesizing, then grounding in graph.

**Example**:
```
User: "How do I implement OAuth2 flows?"

1. Synthesize: "OAuth2 requires [[authorization-server]], [[client-credentials]],
   [[token-endpoint]], and [[JWT]] validation..."

2. Extract: [[authorization-server]], [[client-credentials]], [[token-endpoint]], [[JWT]]

3. buildcontext for each concept (parallel)

4. searchmemories with enriched understanding

5. readmemory for implementation guides

6. Answer grounded in actual memory:// files
```

#### Sequential Thinking Branching

_Sequential Thinking Branching — diagram_ ::
Nodes: Main Session: / sequential-20260129-001 · Thought 1: / Problem analysis · Thought 2: / Approach A or B? · Branch A: / Explore approach A · Branch B: / Explore approach B · Thought A.1: / A seems better · Thought B.1: / B has issues · Thought 3: / Choose A, continue main · Persist to memory:// / with concepts · Graph nodes created.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph TD
    Main[Main Session:<br/>sequential-20260129-001] --> Thought1[Thought 1:<br/>Problem analysis]
    Thought1 --> Thought2[Thought 2:<br/>Approach A or B?]

    Thought2 --> BranchA[Branch A:<br/>Explore approach A]
    Thought2 --> BranchB[Branch B:<br/>Explore approach B]

    BranchA --> ThoughtA1[Thought A.1:<br/>A seems better]
    BranchB --> ThoughtB1[Thought B.1:<br/>B has issues]

    ThoughtA1 --> Merge[Thought 3:<br/>Choose A, continue main]
    ThoughtB1 --> Merge

    Merge --> Memory[Persist to memory://<br/>with concepts]
    Memory --> Graph[Graph nodes created]

    style Main fill:#0969DA
    style Thought1 fill:#0969DA
    style Thought2 fill:#0969DA
    style BranchA fill:#0969DA
    style BranchB fill:#0969DA
    style ThoughtA1 fill:#0969DA
    style ThoughtB1 fill:#0969DA
    style Merge fill:#0969DA
    style Memory fill:#0969DA
    style Graph fill:#0969DA
```

</details>

**Use case**: Explore multiple alternatives without losing thought continuity.

**Example**:
```
1. Start session (thought 0): Set up problem
2. Thought 1: Identify two potential solutions
3. Branch A: Explore solution 1 (thoughts A.1, A.2, A.3)
4. Branch B: Explore solution 2 (thoughts B.1, B.2, B.3)
5. Thought 2 (main): Compare branches, choose best
6. Continue main session with chosen approach
```

### Knowledge Graph Growth

_Knowledge Graph Growth — diagram_ ::
Nodes: User asks question · searchmemories / Check existing knowledge · Use existing / memory:// files · Research external sources · writememory / with concepts · sync / Extract WikiLinks · Create graph nodes / for each concept · Create relationships / from co-occurrence · buildcontext now returns / richer relationships · Future sessions / benefit from growth · Answer question.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph TD
    Start[User asks question] --> Search[searchmemories<br/>Check existing knowledge]

    Search -->|Found| Use[Use existing<br/>memory:// files]
    Search -->|Not found| Research[Research external sources]

    Research --> Write[writememory<br/>with concepts]
    Write --> Sync[sync<br/>Extract WikiLinks]

    Sync --> Nodes[Create graph nodes<br/>for each concept]
    Nodes --> Edges[Create relationships<br/>from co-occurrence]

    Edges --> Context[buildcontext now returns<br/>richer relationships]
    Context --> Future[Future sessions<br/>benefit from growth]

    Use --> Answer[Answer question]
    Future --> Answer

    style Start fill:#0969DA
    style Search fill:#0969DA
    style Use fill:#0969DA
    style Research fill:#0969DA
    style Write fill:#0969DA
    style Sync fill:#0969DA
    style Nodes fill:#0969DA
    style Edges fill:#0969DA
    style Context fill:#0969DA
    style Future fill:#0969DA
    style Answer fill:#0969DA
```

</details>

**Key insight**: The graph is not static. Every interaction can grow it:

1. **Question**: User asks about [[authentication]]
2. **Search**: `searchmemories("authentication")` finds gap
3. **Research**: Query external docs (Microsoft Docs, Context7)
4. **Write**: `writememory` creates lineage-backed note with [[OAuth2]], [[JWT]], [[PKCE]]
5. **Sync**: Extracts all `[[WikiLinks]]` like [[OAuth2]], [[JWT]], [[PKCE]] and creates graph nodes
6. **Growth**: `buildcontext("authentication")` now returns OAuth2, JWT, PKCE relationships
7. **Future**: Next session asking about [[OAuth2]] benefits from prior work

**Compounding effect**: Graph becomes institutional memory that persists and improves across all sessions.

### How Concepts Become Nodes

_How Concepts Become Nodes — diagram_ ::
Nodes: Fixed authentication bug / using JWT tokens / and session-management · sync tool · Graph Node: / authentication · Graph Node: / JWT · Graph Node: / session-management.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph LR
    subgraph "Memory File"
        Content["Fixed authentication bug<br/>using JWT tokens<br/>and session-management"]
    end

    Content --> Sync[sync tool]

    Sync --> Node1["Graph Node:<br/>authentication"]
    Sync --> Node2["Graph Node:<br/>JWT"]
    Sync --> Node3["Graph Node:<br/>session-management"]

    Node1 -.->|co-occurs with| Node2
    Node1 -.->|co-occurs with| Node3
    Node2 -.->|co-occurs with| Node3

    subgraph "Graph Database"
        Node1
        Node2
        Node3
    end

    style Content fill:#0969DA
    style Sync fill:#0969DA
    style Node1 fill:#0969DA
    style Node2 fill:#0969DA
    style Node3 fill:#0969DA
```

</details>

**Process**:
1. Write memory file with `[[WikiLinks]]` in double brackets (e.g., [[authentication]], [[JWT]])
2. Call `sync` (or happens automatically)
3. Sync extracts all `[[WikiLinks]]` from all memory files
4. Creates/updates graph nodes for each unique concept
5. Records co-occurrence relationships (concepts in same file = related)
6. Future `buildcontext` queries traverse these relationships

**Best practices**:
- Use `[[singular-form]]` for general concepts
- Use `[[hyphens-not-spaces]]` for multi-word concepts
- Tag substance, not structure (avoid `[[the]]`, `[[a]]`, `[[file]]`)
- Reuse existing concepts before creating near-duplicates

### Context Compounding Example

_Context Compounding Example — diagram_ ::
Nodes: Session 1: Learn OAuth2 · writememory OAuth2 basics · Session 2: Learn PKCE · buildcontext finds OAuth2 relation · writememory PKCE extension · Session 3: Learn JWT · buildcontext finds OAuth2 + PKCE · writememory JWT tokens · Session 4: Implement auth · buildcontext finds ALL prior work · Rich context: OAuth2, PKCE, JWT, / plus all memory:// files · Grounded implementation.
Relationships shown by arrows in the diagram source below — open in a graphical viewer (or the maenifold desktop app / GitHub web UI) to render.

<details><summary>Mermaid source (graphical viewers only)</summary>

```mermaid
graph TD
    Session1["Session 1: Learn OAuth2"] --> Write1["writememory OAuth2 basics"]

    Session2["Session 2: Learn PKCE"] --> Build2["buildcontext finds OAuth2 relation"]
    Build2 --> Write2["writememory PKCE extension"]

    Session3["Session 3: Learn JWT"] --> Build3["buildcontext finds OAuth2 + PKCE"]
    Build3 --> Write3["writememory JWT tokens"]

    Session4["Session 4: Implement auth"] --> Build4["buildcontext finds ALL prior work"]
    Build4 --> Rich["Rich context: OAuth2, PKCE, JWT,<br/>plus all memory:// files"]

    Rich --> Answer4["Grounded implementation"]

    Write1 --> Session2
    Write2 --> Session3
    Write3 --> Session4

    style Session1 fill:#0969DA
    style Write1 fill:#0969DA
    style Session2 fill:#0969DA
    style Build2 fill:#0969DA
    style Write2 fill:#0969DA
    style Session3 fill:#0969DA
    style Build3 fill:#0969DA
    style Write3 fill:#0969DA
    style Session4 fill:#0969DA
    style Build4 fill:#0969DA
    style Rich fill:#0969DA
    style Answer4 fill:#0969DA
```

</details>

**Timeline**:
- **Day 1**: Learn OAuth2 basics → write memory:// note with [[OAuth2]]
- **Day 2**: Learn PKCE → buildcontext finds [[OAuth2]] relation → write note with [[PKCE]]
- **Day 3**: Learn JWT → buildcontext finds [[OAuth2]] + [[PKCE]] → write note with [[JWT]]
- **Day 4**: Implement auth → buildcontext returns **all 3 concepts + all notes** → rich implementation context

**Result**: Knowledge compounds over time. Later sessions benefit from all prior work automatically.

## Prerequisites

- **Claude Code** (CLI or VS Code extension)
- **maenifold binary** installed and accessible in PATH (or specify full path in configuration)
- **Write permissions** to the maenifold data directory (default: `~/maenifold`)

## Installation

### Method 1: Plugin Installation (Recommended)

Install the maenifold plugin directly through Claude Code:

```bash
claude plugin install msbrettorg/maenifold
```

This automatically:
- Downloads the maenifold plugin
- Configures MCP server settings
- Registers the skill for automatic invocation

### Method 2: Manual Installation

If you prefer manual setup or need custom configuration:

1. Clone or download the plugin files to your `.claude/plugins` directory
2. Configure the MCP server in your project's `.mcp.json` (see MCP Configuration below)
3. Copy skill files to your project or reference them from the plugin cache

## Binary Installation

The maenifold MCP server requires the maenifold binary. Choose your installation method:

### GitHub Releases (Recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/msbrettorg/maenifold/releases/latest).

#### macOS

1. Download the appropriate archive:
   - **Apple Silicon (M1/M2/M3)**: `maenifold-osx-arm64.tar.gz`
   - **Intel**: `maenifold-osx-x64.tar.gz`

2. Extract to `~/maenifold/bin`:
   ```bash
   # Download (replace URL with correct architecture)
   curl -L -o maenifold.tar.gz https://github.com/msbrettorg/maenifold/releases/latest/download/maenifold-osx-arm64.tar.gz

   # Extract to bin directory
   mkdir -p ~/maenifold/bin
   tar -xzf maenifold.tar.gz -C ~/maenifold/bin --strip-components=1
   ```

3. Add to PATH:
   ```bash
   # Add to ~/.zshrc or ~/.bash_profile
   echo 'export PATH="$HOME/maenifold/bin:$PATH"' >> ~/.zshrc

   # Reload shell configuration
   source ~/.zshrc
   ```

4. Verify installation:
   ```bash
   maenifold --tool GetConfig --payload '{}'
   ```

#### Linux

1. Download `maenifold-linux-x64.tar.gz`

2. Extract to `~/maenifold/bin`:
   ```bash
   # Download
   curl -L -o maenifold.tar.gz https://github.com/msbrettorg/maenifold/releases/latest/download/maenifold-linux-x64.tar.gz

   # Extract to bin directory
   mkdir -p ~/maenifold/bin
   tar -xzf maenifold.tar.gz -C ~/maenifold/bin --strip-components=1
   ```

3. Add to PATH:
   ```bash
   # Add to ~/.bashrc or ~/.zshrc
   echo 'export PATH="$HOME/maenifold/bin:$PATH"' >> ~/.bashrc

   # Reload shell configuration
   source ~/.bashrc
   ```

4. Verify installation:
   ```bash
   maenifold --tool GetConfig --payload '{}'
   ```

#### Windows

1. Download `maenifold-win-x64.zip`

2. Extract to `%USERPROFILE%\maenifold\bin`:
   ```powershell
   # Download (adjust path to your Downloads folder)
   $downloadPath = "$env:USERPROFILE\Downloads\maenifold-win-x64.zip"

   # Create directory and extract
   New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\maenifold\bin"
   Expand-Archive -Path $downloadPath -DestinationPath "$env:USERPROFILE\maenifold\bin" -Force
   ```

3. Add to PATH:
   ```powershell
   # Add to user PATH
   $path = [Environment]::GetEnvironmentVariable("PATH", "User")
   $newPath = "$env:USERPROFILE\maenifold\bin"
   if ($path -notlike "*$newPath*") {
       [Environment]::SetEnvironmentVariable("PATH", "$path;$newPath", "User")
   }

   # Restart terminal for PATH change to take effect
   ```

4. Verify installation:
   ```powershell
   maenifold --tool GetConfig --payload '{}'
   ```

### Homebrew (Alternative for macOS/Linux)

If you prefer using Homebrew package manager:

```bash
brew install msbrettorg/tap/maenifold
```

This installs the binary and adds it to your PATH automatically. Note that the default installation location differs from the recommended `~/maenifold` directory.

### Build from Source

Requires .NET 9.0 SDK.

```bash
git clone https://github.com/msbrettorg/maenifold.git
cd maenifold
dotnet build -c Release
```

The binary will be at `src/Maenifold/bin/Release/net9.0/maenifold` (or `maenifold.exe` on Windows). Copy to `~/maenifold/bin` and add to PATH as shown in the platform-specific instructions above:

```bash
# macOS/Linux
mkdir -p ~/maenifold/bin
cp src/Maenifold/bin/Release/net9.0/maenifold ~/maenifold/bin/
```

## MCP Configuration

The maenifold skill requires Copilot CLI to connect to the running maenifold HTTP daemon. The expected Streamable HTTP MCP endpoint is:

- `http://127.0.0.1:8080/mcp`

Start the daemon through the repo-supported HTTP path before opening Copilot:

```bash
maenifold --http
```

That command hosts maenifold MCP on port `8080` by default. It is also the host for native Gateway domains.

### Copilot CLI

Configure the user-level Copilot MCP file at `~/.copilot/mcp-config.json`. Do not mutate this file from the plugin installer; document the following entry for the user's active Copilot configuration:

```json
{
  "mcpServers": {
    "maenifold": {
      "url": "http://127.0.0.1:8080/mcp",
      "type": "http",
      "tools": ["*"]
    }
  }
}
```

Verify that Copilot loaded the active user-level config:

```bash
copilot mcp list
```

The output should include `maenifold (http)`. If `copilot mcp list` does not show plugin servers, this plugin's `.mcp.json` is insufficient by itself; add the `~/.copilot/mcp-config.json` entry above.

### Gate tools

Gate is not a separate MCP server for Copilot. Do not configure a `gate` MCP server entry, a second port, or standalone Gate daemon flags. Gate operations are native Gateway tools served through maenifold on the same `http://127.0.0.1:8080/mcp` endpoint.

The Gate Gateway domain is `maenifold.gate`, with these commands:

- `gate_Status`
- `gate_Pass`
- `gate_Fail`
- `gate_Append`
- `gate_Sync`
- `gate_History`
- `gate_TaskBody`

Any plugin or config example that declares `gate` as a separate MCP server, or starts Gate as its own MCP/HTTP process, is NO LONGER VALID — replaced by the `maenifold.gate` domain on port `8080`.

The plugin `.mcp.json` mirrors the same single maenifold HTTP server shape for clients that load plugin-local MCP files. Copilot CLI users should still verify with `copilot mcp list`.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MAENIFOLD_ROOT` | Root directory for memory files and database | `~/maenifold` |

Set the environment variable to use a custom location:

```bash
# Bash/Zsh
export MAENIFOLD_ROOT=~/my-knowledge-base

# PowerShell
$env:MAENIFOLD_ROOT = "$HOME\my-knowledge-base"
```

Or export it in the shell before starting the maenifold HTTP daemon.


## Verification

After installation, verify the setup:

### 1. Check Binary Installation

```bash
maenifold --tool GetConfig --payload '{}'
```

Expected output: JSON configuration including `maenifoldRoot` path and version info

### 2. Test HTTP MCP Server

```bash
maenifold --http
```

The server should expose Streamable HTTP MCP at `http://127.0.0.1:8080/mcp`. Keep this daemon running while Copilot uses the tools.

### 3. Verify Copilot MCP Registration

```bash
copilot mcp list
```

Expected output includes `maenifold (http)`. If it does not, the active user-level Copilot config did not load the maenifold HTTP server entry.

### 4. Test from Copilot CLI

Start Copilot CLI and try:

```
Write a test memory about [[installation]] verification
```

If successful, you should see confirmation that the memory was written.

### 5. Verify Skill Loading

In Copilot CLI, the skill should load automatically when you mention:
- "write memory", "read memory", "search memories"
- "build context", "find similar concepts"
- "sequential thinking", "run workflow"
- `[[WikiLinks]]`, `memory://`, or maenifold tools

## Troubleshooting

### "Command not found" Error

- **Cause**: maenifold binary not in PATH
- **Solution**: Either add the installation directory to PATH or use the full path in MCP configuration

### "Permission denied" Error

- **Cause**: Cannot write to MAENIFOLD_ROOT directory
- **Solution**: Ensure the directory exists and you have write permissions:
  ```bash
  mkdir -p ~/maenifold
  chmod 755 ~/maenifold
  ```

### MCP Server Timeout

- **Cause**: Server taking too long to start
- **Solution**: Start the maenifold HTTP daemon before launching Copilot and verify `copilot mcp list` reports `maenifold (http)`.

### Database Lock Error

- **Cause**: Multiple processes accessing the same database
- **Solution**: Ensure only one MCP server instance is running per MAENIFOLD_ROOT. The system supports concurrent agents, but each must use the same server instance.

### Skill Not Loading

- **Cause**: Skill files not found or plugin not installed
- **Solution**:
  1. Verify the maenifold plugin is installed in Copilot CLI
  2. Verify `copilot mcp list` includes `maenifold (http)`

### Empty Search Results

- **Cause**: Graph not synced after adding memories
- **Solution**: Run the sync tool:
  ```
  Sync the knowledge graph
  ```

## Copilot Bootstrap Summary

| Requirement | Value |
|-------------|-------|
| MCP endpoint | `http://127.0.0.1:8080/mcp` |
| Copilot config path | `~/.copilot/mcp-config.json` |
| Server entry | `maenifold` with `type: "http"` |
| Gate exposure | Native `maenifold.gate` Gateway tools through maenifold |
| Verification | `copilot mcp list` shows `maenifold (http)` |

## Related Documentation

- [Main README](https://github.com/msbrettorg/maenifold/blob/main/README.md) - Project overview and quick start
- [Full Documentation](https://github.com/msbrettorg/maenifold/blob/main/docs/README.md) - Architecture and examples
- [GitHub Releases](https://github.com/msbrettorg/maenifold/releases) - Download binaries
- [SKILL.md](../../skills/maenifold/SKILL.md) - Skill usage and tool reference

## Tool Reference

For detailed tool documentation, see the `../../skills/maenifold/references/` directory:

| Category | Tools |
|----------|-------|
| Memory | `writememory`, `readmemory`, `searchmemories`, `editmemory`, `deletememory`, `movememory`, `listmemories` |
| Graph | `buildcontext`, `findsimilarconcepts`, `visualize`, `sync`, `extractconceptsfromfile` |
| Repair | `analyzeconceptcorruption`, `repairconcepts` |
| Reasoning | `workflow`, `sequentialthinking`, `assumptionledger` |
| System | `adopt`, `getconfig`, `gethelp`, `memorystatus`, `recentactivity`, `updateassets` |
