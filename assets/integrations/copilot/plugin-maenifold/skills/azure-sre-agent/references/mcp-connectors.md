# MCP Connectors Deep Reference

Detailed reference for Azure SRE Agent's MCP connector system including architecture, transport types, health monitoring, and tool management.

## Architecture

MCP (Model Context Protocol) is an open standard for connecting AI agents to external tools. An MCP server wraps any service and exposes capabilities as discoverable tools.

### Connection Lifecycle

1. **Auto-discovery** — Agent calls server's tool listing, knows available tools immediately
2. **Namespaced registration** — Tools prefixed with connection ID (e.g., `my-datadog_list_metrics`)
3. **Health monitoring** — 60-second heartbeat pings, auto-recovery from transient failures
4. **Dynamic updates** — New server tools detected automatically within 5 minutes

## Transport Types

### Streamable-HTTP (Remote)

For cloud-hosted services and SaaS platforms.

**Configuration:**
- URL endpoint
- Authentication: Bearer token or Custom headers
- Used by: Datadog, GitHub, Splunk, New Relic, Dynatrace, Elasticsearch

### Stdio (Local Processes)

For processes running inside the agent container.

**Supported runtimes:**
- `npx`, `node` → Node.js 20
- `python`, `python3` → Python 3.12
- `dotnet` → .NET 9

**NOT supported:** Docker containers, arbitrary native binaries.

The agent manages the process lifecycle — starting on connection, monitoring health, restarting on failure.

## Pre-configured Partner Connectors

| Partner | Transport | What You Provide | What Agent Gets |
|---------|-----------|-----------------|-----------------|
| GitHub | Streamable-HTTP | PAT | Repos, PRs, issues, code search |
| Datadog | Streamable-HTTP | API Key + App Key | Metrics, APM, alerts |
| New Relic | Streamable-HTTP | API Key | Performance monitoring |
| Splunk | Streamable-HTTP | Bearer token | Log search, events |
| Dynatrace | Streamable-HTTP | Bearer token | APM |
| Elasticsearch | Streamable-HTTP | Auth header | Search, analytics, logs |
| Hawkeye (NeuBird) | Streamable-HTTP | Bearer token | AI observability |

**Finding more:** [Azure MCP Center](https://mcp.azure.com) for verified servers.

## Tool Management

### Tool Budget

**80 tools maximum** per agent (native + MCP combined). The budget spans all connectors.

| Tool Count | Bar Color | Meaning |
|-----------|-----------|---------|
| 0–56 (≤70%) | Blue | Plenty of room |
| 57–72 (71–90%) | Yellow | Approaching limit |
| 73–80 (>90%) | Red | Near/at capacity |

### At Capacity

- Unchecked tools become disabled
- Already-checked tools remain enabled (deselect to free space)
- "Select all" caps at remaining capacity

### Agent vs Custom Agent Tools

| Approach | When to Use |
|----------|-------------|
| **Agent tools** | Selected during connector setup; available directly in main chat |
| **Custom agent tools** | Assigned to specific subagents for focused specialists |

The same tool can be visible to both — no conflict.

### YAML Tool Assignment

**Individual tools:**
```yaml
mcp_tools:
  - kusto-mcp_kusto_query
  - kusto-mcp_kusto_table_list
```

**Wildcard (all tools from server):**
```yaml
mcp_tools:
  - kusto-mcp/*
```

**Mixed:**
```yaml
mcp_tools:
  - kusto-mcp/*              # All from Kusto
  - grafana-mcp_dashboard    # One from Grafana
```

**Important:** Wildcard must use `{connection-id}/*` with forward slash. `kusto-mcp*` is treated as exact name.

## Health Monitoring

| Status | Meaning | Action |
|--------|---------|--------|
| Connected | Healthy, tools ready | None |
| Disconnected | Temporary loss, auto-recovery in progress | Usually none |
| Failed | Unrecoverable error | Check URL, credentials, network |
| Initializing | Being established | Wait |
| Error/Not Available | No running agent instance | Start the agent |

### Auto-Recovery

- Heartbeat: every 60 seconds
- Transient failures: next successful heartbeat restores
- Pre-execution: validates connection, attempts reconnect before tool use

### When Auto-Recovery Can't Help

| Situation | What to Do |
|-----------|-----------|
| Network blip | Nothing — automatic |
| Server temporarily down | Wait for return |
| Wrong URL/credentials | Update connector config |
| Server permanently removed | Delete and re-create |

## Authentication Methods

| Method | When | How |
|--------|------|-----|
| Bearer token | Most SaaS APIs | Token in Authorization header |
| Custom headers | APIs requiring specific headers | Arbitrary key-value pairs |
| Managed identity | Azure services via stdio | Azure AD tokens via agent identity |

## Setup Tutorial

Quick steps:
1. **Builder → Connectors → + Add connector**
2. Select partner card or Custom
3. Enter credentials
4. Wait for **Connected** status
5. Select tools
6. Test in a new chat thread

## For More Details

- `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/mcp-connectors")`
- `microsoft_docs_fetch(url="https://sre.azure.com/docs/tutorials/connectors/setup-mcp-connector")`
- `microsoft_docs_search(query="Azure SRE Agent MCP partner connectors Datadog Splunk")`
