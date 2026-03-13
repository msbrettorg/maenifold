# Maenifold Integration with Azure SRE Agent

This guide covers deploying maenifold as an MCP connector for Azure SRE Agent, enabling persistent knowledge graph capabilities alongside SRE Agent's built-in memory system.

## Why Integrate Maenifold

SRE Agent has its own memory system (session insights, user memories, knowledge base). Maenifold adds:

- **Cross-agent knowledge graph** — concepts linked via `[[WikiLinks]]` persist across all agents (SRE Agent, Copilot CLI, Claude, etc.)
- **Sequential thinking** — persistent hierarchical state machines for structured reasoning
- **69 composable workflows** — from deductive reasoning to multi-agent sprints
- **Hybrid search** — semantic vectors + full-text with RRF fusion
- **Community detection** — automatic clustering of reasoning domains
- **Decay weighting** — ACT-R power-law recency bias surfaces fresh knowledge

## Integration Architecture

```
┌─────────────────────┐     Streamable-HTTP      ┌──────────────────┐
│   Azure SRE Agent   │ ◄──────────────────────► │    maenifold     │
│                     │    (MCP connector)        │  (HTTP endpoint) │
│  ┌───────────────┐  │                           │                  │
│  │ Built-in      │  │                           │  ┌────────────┐  │
│  │ Memory System │  │                           │  │ Knowledge  │  │
│  └───────────────┘  │                           │  │ Graph DB   │  │
│  ┌───────────────┐  │                           │  └────────────┘  │
│  │ MCP Tools     │  │                           │  ┌────────────┐  │
│  │ (maenifold/*) │  │                           │  │ Memory     │  │
│  └───────────────┘  │                           │  │ Files (md) │  │
└─────────────────────┘                           │  └────────────┘  │
                                                  └──────────────────┘
                                                  Deployed on:
                                                  - Azure Container Apps
                                                  - Azure Container Instance
                                                  - Any HTTP-accessible host
```

## Deployment Option: Streamable-HTTP

SRE Agent stdio MCP connectors only support `node`, `python`, and `dotnet` runtimes. Since maenifold is a native Rust binary, connect via **Streamable-HTTP** transport.

### Step 1: Deploy Maenifold as an HTTP MCP Server

Deploy the maenifold binary to an HTTP-accessible environment. Options:

**Azure Container Apps (recommended):**

```bash
# Build a container image with maenifold
# Dockerfile example:
# FROM mcr.microsoft.com/cbl-mariner/base/core:2.0
# COPY maenifold /usr/local/bin/maenifold
# COPY memory/ /data/memory/
# ENV MAENIFOLD_ROOT=/data
# EXPOSE 8080
# CMD ["maenifold", "--mcp", "--http", "--port", "8080"]

az containerapp create \
  --name maenifold-mcp \
  --resource-group rg-sre-agent \
  --environment sre-agent-env \
  --image your-registry.azurecr.io/maenifold:latest \
  --target-port 8080 \
  --ingress external \
  --min-replicas 1 \
  --max-replicas 1 \
  --env-vars \
    MAENIFOLD_ROOT=/data \
    MAENIFOLD_AUTO_SYNC=true
```

**Azure Container Instance (simpler, lower cost):**

```bash
az container create \
  --resource-group rg-sre-agent \
  --name maenifold-mcp \
  --image your-registry.azurecr.io/maenifold:latest \
  --ports 8080 \
  --environment-variables \
    MAENIFOLD_ROOT=/data \
    MAENIFOLD_AUTO_SYNC=true \
  --azure-file-volume-account-name storageaccount \
  --azure-file-volume-account-key <key> \
  --azure-file-volume-share-name maenifold-data \
  --azure-file-volume-mount-path /data
```

### Step 2: Configure MCP Connector in SRE Agent

1. Go to [sre.azure.com](https://sre.azure.com) → select your agent
2. Navigate to **Builder → Connectors**
3. Click **+ Add connector** → Select **Custom MCP server**
4. Configure:

| Field | Value |
|-------|-------|
| Name | `maenifold-mcp` |
| Transport | Streamable-HTTP |
| URL | `https://maenifold-mcp.<region>.azurecontainerapps.io/mcp/` |
| Auth | Bearer token (if configured) or None |

5. Click **Next** → Review → **Add connector**
6. Wait for status: **Connected** (green checkmark)
7. Select tools to make available to the agent

### Step 3: Select Maenifold Tools

After connecting, select which maenifold tools the agent can use. Recommended core tools:

| Tool | Purpose | Priority |
|------|---------|----------|
| `SearchMemories` | Find knowledge across the graph | Essential |
| `BuildContext` | Traverse concept relationships | Essential |
| `WriteMemory` | Persist new knowledge | Essential |
| `ReadMemory` | Read specific memory files | Essential |
| `EditMemory` | Update existing knowledge | High |
| `FindSimilarConcepts` | Semantic concept discovery | High |
| `SequentialThinking` | Structured reasoning sessions | Medium |
| `Workflow` | Run structured workflows | Medium |
| `Sync` | Force graph synchronization | Medium |
| `MemoryStatus` | Check system health | Low |

**Budget note:** These 10 tools use 10 of your 80-tool budget. Plan accordingly if you have other connectors.

### Step 4: Create a Maenifold Subagent (Optional)

For advanced use, create a dedicated subagent that routes knowledge graph operations:

```yaml
name: knowledge_engineer
system_prompt: |
  You are a knowledge graph specialist using maenifold.
  When asked to research a topic:
  1. Use SearchMemories to find existing knowledge
  2. Use BuildContext to understand concept relationships
  3. Use FindSimilarConcepts for related concepts
  4. Write findings with WriteMemory using [[WikiLinks]]
  
  Always ground answers in the knowledge graph. Never rely
  on internal memory alone.
handoff_description: Manages persistent knowledge graph, researches topics, writes structured memory
mcp_tools:
  - maenifold-mcp/*
```

## Complementary Memory Systems

SRE Agent and maenifold have complementary memory systems:

| Feature | SRE Agent Memory | Maenifold |
|---------|-----------------|-----------|
| **Scope** | Per-agent, operational focus | Cross-agent, cross-domain |
| **Auto-learning** | Session insights (30 min delay) | WikiLink extraction on write |
| **Search** | Document search + vector | Hybrid (semantic + full-text + RRF) |
| **Structure** | Flat files with overview.md | Graph with community detection |
| **Decay** | No built-in decay | ACT-R power-law recency |
| **Reasoning** | Agent's built-in planning | Sequential thinking + workflows |
| **Access** | Single SRE Agent instance | Any MCP-connected client |

**Best practice:** Use SRE Agent's built-in memory for operational context (incident learnings, resource-specific knowledge). Use maenifold for cross-cutting institutional knowledge that should be available to all agents and human operators.

## Stdio Support (Future)

Currently, SRE Agent stdio supports only Node.js, Python, and .NET. If stdio expands to support arbitrary binaries or Rust runtimes, maenifold could connect directly:

```yaml
# FUTURE - not currently supported
# Hypothetical stdio configuration
connector:
  type: stdio
  command: maenifold
  args: ["--mcp"]
  env:
    MAENIFOLD_ROOT: /data/maenifold
```

Monitor [SRE Agent release notes](https://sre.azure.com/docs/) for runtime expansion announcements.

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Connector shows **Failed** | Verify maenifold HTTP endpoint is reachable, check URL and auth |
| Tools not appearing | Ensure connector status is **Connected**, then edit connector to select tools |
| Knowledge not persisting | Check `MAENIFOLD_ROOT` is mounted to persistent storage (Azure Files) |
| Slow search responses | Consider deploying maenifold closer to SRE Agent region |
| Graph out of sync | Use `Sync` tool or verify `MAENIFOLD_AUTO_SYNC=true` |
