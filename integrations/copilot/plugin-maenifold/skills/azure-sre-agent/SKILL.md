---
name: azure-sre-agent
description: Work with Azure SRE Agent — create, deploy, manage, and extend AI-powered reliability agents. Use when agents need to provision SRE Agents, author skills/subagents, configure MCP connectors (including maenifold), manage memory and knowledge, set up incident response, or deploy infrastructure with Bicep. Covers the full lifecycle from deployment through day-2 operations.
---

# Azure SRE Agent

Azure SRE Agent is an AI-powered reliability assistant that automates operational work, reduces toil, and lowers mean time to resolution (MTTR). It connects to Azure resources, observability tools, incident platforms, and source code repositories — then helps teams respond faster, investigate deeper, and automate reliably.

**Portal**: [sre.azure.com](https://sre.azure.com)
**Docs**: [sre.azure.com/docs](https://sre.azure.com/docs/) and [learn.microsoft.com/azure/sre-agent](https://learn.microsoft.com/en-us/azure/sre-agent/)
**GitHub**: [github.com/microsoft/sre-agent](https://github.com/microsoft/sre-agent)

## Key Concepts

### Agent

The core SRE Agent instance deployed in your Azure subscription. It has built-in access to Azure Monitor, Application Insights, Log Analytics, Azure Resource Graph, and Azure CLI. No connectors required for Azure-native operations.

**Created resources on deployment:**
- Managed Identity (authenticates to Azure services)
- Log Analytics Workspace (agent telemetry)
- Application Insights (agent health monitoring)
- Role Assignments (Reader on selected subscriptions/resource groups)

### Skills

Reusable procedures with optional tool attachments. The agent loads them **automatically** when relevant — no `/skill` command needed. Skills combine a `SKILL.md` (Markdown instructions with YAML frontmatter) with optional tools (Azure CLI, Kusto, Python, MCP).

**Limits:** Max 5 concurrent active skills. Oldest auto-unloaded when exceeded.

### Custom Agents (Subagents)

Specialist agents invoked on demand via `/agent` command. They share the full conversation context. Custom agents have their own instructions, tools, skills, and operate in Review or Autonomous mode.

**Patterns:** Domain Expert (VM Expert, AKS Expert), Task Specialist (Log Analyzer, Cost Optimizer), Workflow Executor (Incident Triage, Deployment Validator).

### Connectors

Extend agent reach beyond Azure. Four categories:
- **Data sources**: Azure Data Explorer / Kusto
- **Source code & knowledge**: GitHub MCP, Azure DevOps OAuth
- **Collaboration**: Teams, Outlook
- **Custom (MCP)**: Any MCP-compatible server via Streamable-HTTP or stdio

### Memory System

Three knowledge layers that build automatically:
- **Session Insights**: Auto-extracted after each thread (symptoms, resolution, root cause, pitfalls). Generated 30 min after thread goes quiet.
- **User Memories**: Explicit facts saved with `#remember`, retrieved with `#retrieve`, deleted with `#forget`.
- **Knowledge Base**: Uploaded documents (runbooks, architecture guides) plus connected external sources.

**Proactive Knowledge Persistence**: The agent maintains `memories/synthesizedKnowledge/` with an `overview.md` (always loaded into context, ~2000 char budget) and topic files linked from it.

### Deep Context

The agent's accumulated understanding built through three pillars:
1. **Context Analysis** — reads code, searches knowledge, navigates your environment in real time
2. **Persistent Memory** — remembers past investigations, team context, operational patterns
3. **Background Intelligence** — continuous codebase analysis, insight generation, Kusto schema enrichment

## Getting Started

### Prerequisites

| Requirement | Details |
|-------------|---------|
| Azure subscription | Contributor role (Owner/UAA for RBAC in managed resource groups) |
| Network access | `*.azuresre.ai` reachable from browser |
| Region | **East US 2**, **Sweden Central**, or **Australia East** ONLY |
| Resource provider | `Microsoft.App` registered on subscription |

### Deploy via Portal

1. Navigate to [sre.azure.com](https://sre.azure.com) → sign in
2. Fill in: Subscription, Resource Group, Agent Name, Region
3. Review → **Create** (deploys in 2-5 minutes)
4. **Set up**: Connect code repos → Add Azure resource access → Done

### Deploy via Bicep (Infrastructure as Code)

The SRE Agent resource type is `Microsoft.App/agents@2025-05-01-preview`.

See [sample_codes/getting-started/deploy-sre-agent.bicep](sample_codes/getting-started/deploy-sre-agent.bicep) for the complete template.

```bash
az deployment group create \
  --resource-group rg-sre-agent \
  --template-file deploy-sre-agent.bicep \
  --parameters agentName='my-sre-agent' \
               appInsightsAppId='<app-id>' \
               appInsightsConnectionString='<conn-string>'
```

**Key Bicep properties:**

```bicep
#disable-next-line BCP081
resource sreAgent 'Microsoft.App/agents@2025-05-01-preview' = {
  name: agentName
  location: location   // eastus2, swedencentral, or australiaeast
  identity: {
    type: 'SystemAssigned, UserAssigned'
    userAssignedIdentities: { '${managedIdentity.id}': {} }
  }
  properties: {
    knowledgeGraphConfiguration: {
      managedResources: [ resourceGroupId ]
      identity: managedIdentity.id
    }
    actionConfiguration: {
      mode: 'Review'      // or 'autonomous'
      identity: managedIdentity.id
      accessLevel: 'High' // or 'Low'
    }
    mcpServers: []
    logConfiguration: {
      applicationInsightsConfiguration: {
        appId: appInsightsAppId
        connectionString: appInsightsConnectionString
      }
    }
  }
}
```

**SRE Agent Administrator role ID:** `e79298df-d852-4c6d-84f9-5d13249d1e55` — assign to deploying user for portal access.

### Deploy via Azure Developer CLI (azd)

For a full lab environment (agent + sample app + monitoring + subagents):

```bash
git clone https://github.com/dm-chelupati/sre-agent-lab.git
cd sre-agent-lab
azd auth login && azd up
```

Deploys in ~8-12 minutes. Includes pre-configured subagents, knowledge base, and incident response plans.

### Deploy Teams Bot via Bicep

See [sample_codes/getting-started/deploy-teams-bot.bicep](sample_codes/getting-started/deploy-teams-bot.bicep)

Required values: Bot App Client ID (managed identity), Bot App Domain (container app FQDN), Tenant ID.

## Skills Authoring

### Skill Structure

```yaml
name: high-cpu-troubleshooting
description: Troubleshooting procedure for high CPU alerts on container apps
tools:
  - RunAzCliReadCommands
```

```markdown
## When to use this skill
Use this skill when you receive a high CPU alert on any container app.

## Steps
1. Check upstream dependencies for cascading failures
2. Query connection pool metrics for the last hour
3. Review deployments in the last 24 hours
4. If a recent deployment correlates with CPU spike, identify the commit
5. Recommend rollback or fix based on findings
```

Create in **Builder → Agent Canvas → Create → Skill**. The agent reads skill descriptions and auto-activates the most relevant one.

**Tool types attachable to skills:** Azure CLI, Kusto queries, Python scripts, MCP tools, Link (URL templates).

See [sample_codes/common-patterns/skill-yaml.md](sample_codes/common-patterns/skill-yaml.md) for more examples.

## Custom Agent Authoring

### YAML Definition

The `azuresre.ai/v1` API format is used for subagent configs deployable via REST API:

```yaml
api_version: azuresre.ai/v1
kind: AgentConfiguration
spec:
  name: incident-handler
  system_prompt: |
    You are an expert in triaging and diagnosing incidents. When triggered,
    search the knowledge base for the relevant runbook, execute the diagnostic
    steps, collect evidence, and provide a summary with your findings.
    Always search memory for similar past incidents first.
    Use ExecutePythonCode to plot metrics charts when presenting evidence.
  handoff_description: Investigates incidents using runbooks and provides findings
  agent_type: Autonomous
  tools:
    - SearchMemory
    - RunAzCliReadCommands
    - RunAzCliWriteCommands
    - GetAzCliHelp
    - QueryLogAnalyticsByWorkspaceId
    - QueryAppInsightsByResourceId
    - ExecutePythonCode
```

For portal-based creation, the equivalent fields map to the UI form.

**Key properties:**
- `system_prompt` — Expert persona and instructions
- `handoff_description` — What orchestrator sees when delegating
- `tools` — Available capabilities (see [Built-in Tools](#built-in-tools-no-setup-required))
- `agent_type` — `Autonomous` or `Review`
- `mcp_tools` — MCP tool assignments (supports `connection-id/*` wildcards)

### Subagent with MCP Tools (Code Analyzer)

```yaml
api_version: azuresre.ai/v1
kind: AgentConfiguration
spec:
  name: code-analyzer
  system_prompt: |
    You are an expert in diagnosing and finding root cause in applications.
    1. Search the knowledge base for the relevant runbook and execute ALL
       diagnostic steps — query logs, check metrics, collect evidence.
    2. Use source code to correlate log errors to specific code paths.
       Provide file:line references to the root cause.
    3. Create a GitHub issue with detailed findings including log evidence,
       metrics, code snippets, suggested fixes, and remediation actions.
    Always search memory for similar past incidents first.
  handoff_description: Deep root cause analysis using runbooks + source code
  agent_type: Autonomous
  tools:
    - SearchMemory
    - RunAzCliReadCommands
    - QueryLogAnalyticsByWorkspaceId
    - QueryAppInsightsByResourceId
    - ExecutePythonCode
  mcp_tools:
    - github-mcp/*
```

### MCP Tool Assignment

```yaml
mcp_tools:
  - datadog-mcp/*           # All tools from Datadog connection
  - github_search_code      # One specific GitHub tool
```

### Connector YAML (for REST API deployment)

```yaml
api_version: azuresre.ai/v1
kind: DataConnector
spec:
  name: github-mcp
  type: Mcp
  endpoint: https://api.githubcopilot.com/mcp/
  auth:
    type: BearerToken
    token: <GITHUB_PAT>
```

See [sample_codes/common-patterns/subagent-yaml.md](sample_codes/common-patterns/subagent-yaml.md) for patterns.

## MCP Connectors

### Streamable-HTTP (Remote Services)

Connect to any MCP server accessible via URL. Used for SaaS platforms (Datadog, GitHub, Splunk, New Relic).

**Setup:** Builder → Connectors → + Add connector → Select partner or Custom → Enter URL + credentials → Select tools

### Stdio (Local Processes)

Run MCP servers as processes inside the agent container.

**Supported runtimes:**
| Command | Runtime |
|---------|---------|
| `npx`, `node` | Node.js 20 |
| `python`, `python3` | Python 3.12 |
| `dotnet` | .NET 9 |

Docker containers NOT supported. Only pre-installed runtimes available.

### Partner Connectors (Pre-configured)

| Partner | Transport | Auth |
|---------|-----------|------|
| GitHub | Streamable-HTTP | PAT (Bearer) |
| Datadog | Streamable-HTTP | API Key + App Key |
| New Relic | Streamable-HTTP | API Key |
| Splunk | Streamable-HTTP | Bearer token |
| Dynatrace | Streamable-HTTP | Bearer token |
| Elasticsearch | Streamable-HTTP | Auth header |

### Tool Budget

**80 tools maximum** per agent (native + MCP combined). Capacity bar shows color-coded warnings: Blue (≤70%), Yellow (71-90%), Red (>90%).

### Health Monitoring

60-second heartbeat pings. Statuses: Connected, Disconnected (auto-recovery), Failed (check config), Initializing.

### Connecting Maenifold as an MCP Server

See [references/maenifold-integration.md](references/maenifold-integration.md) for the complete guide on deploying maenifold as an MCP connector for SRE Agent.

**Summary:** Since SRE Agent stdio only supports Node.js/Python/.NET, maenifold (a Rust binary) connects via **Streamable-HTTP** transport. Deploy maenifold behind an HTTP endpoint (Azure Container Apps or similar), then configure as a custom MCP connector.

## Memory & Knowledge Management

### Automatic Learning

After each thread (30 min after last message), the agent extracts:
- Symptoms observed (error messages, patterns)
- Steps that worked (resolution path)
- Root cause (what caused the issue)
- Pitfalls to avoid (dead ends)

### Knowledge File Organization

| File | Content | Budget |
|------|---------|--------|
| `overview.md` | Service summary, key links, index | ~2,000 chars |
| `team.md` | Members, roles, expertise | ~500 chars |
| `architecture.md` | Components, connections, environments | ~1,500 chars |
| `logs.md` | Log sources, tables, key fields | ~1,500 chars |
| `deployment.md` | Pipeline details, rollback procedures | ~1,000 chars |
| `debugging.md` | Common issues, troubleshooting guides | ~1,000 chars |
| `queries/*.md` | Extracted queries by topic | ~1,000 chars each |

### User Memory Commands

| Command | Purpose | Example |
|---------|---------|---------|
| `#remember` | Save a fact | `#remember our Redis cache uses Premium tier` |
| `#retrieve` | Search saved memories | `#retrieve what's our caching setup?` |
| `#forget` | Remove a memory | `#forget the outdated Redis info` |

### Upload Knowledge Documents

**Builder → Knowledge base**. Supported: Markdown, plain text, PDF, Word, PowerPoint, Excel, images. Max 16 MB per file.

## Built-in Tools (No Setup Required)

Your agent has these tools via managed identity + RBAC:

| Category | Tools |
|----------|-------|
| **Compute** | VM diagnostics, App Service, Container Apps, AKS kubectl, Functions |
| **Monitoring** | Azure Monitor metrics, Application Insights queries, Log Analytics |
| **Discovery** | Azure Resource Graph, ARM resource operations |
| **Code** | Python interpreter (sandboxed), shell commands |
| **Knowledge** | Document search, vector search, topology mapping |

### Code Interpreter Constraints

Runs in isolated Azure Container Apps session:
- No network access (no HTTP/HTTPS)
- No process spawning (no subprocess/os.system)
- No package installation
- Files must be saved to `/mnt/data/`
- Pre-installed: pandas, matplotlib, seaborn

## Best Practices

- **Connect your most incident-prone repository first** — it immediately starts building Deep Context
- **Name documents clearly** — `production-database-failover.md` not `doc1.txt`
- **Review knowledge quarterly** — outdated docs cause incorrect responses
- **Use skills for procedures, knowledge docs for reference** — don't mix them
- **Test skills and subagents in the Playground** before production
- **Max 5 concurrent skills** — prioritize the most commonly needed procedures
- **80-tool budget** — choose connector tools deliberately; use wildcards for trusted servers

## Common Patterns

### Incident Response Automation

1. Connect incident platform (PagerDuty/ServiceNow) → Configure incident response plan
2. Create specialized subagent for triage with relevant tools
3. Set mode: Review (human approval) or Autonomous (trusted operations)
4. Test in Playground before activating

### Scheduled Health Checks

Create a subagent → Link to scheduled task → Define schedule → Set custom instructions

### Knowledge-Enhanced Troubleshooting

Upload runbooks → Create skill with step-by-step procedures → Attach Azure CLI tools → Agent auto-applies during relevant incidents

## Community & Ecosystem

See [references/community-and-ecosystem.md](references/community-and-ecosystem.md) for the full catalog of:
- **Official repos**: microsoft/sre-agent (community hub), Azure/sre-agent-plugins (plugin marketplace)
- **Sandbox labs**: 10+ breakable AKS scenarios (matthansen0), azd-based lab with subagents (dm-chelupati)
- **Demo environments**: Proactive + reactive demos with Terraform (jiratouchmhp)
- **Cost estimates** and **deployment pattern comparison**
- **Plugin development** templates and marketplace

## Learn More

| Topic | How to Find |
|-------|-------------|
| Bicep resource type | `microsoft_docs_search(query="Microsoft.App agents Bicep template 2025-05-01-preview")` |
| Detailed connector setup | `microsoft_docs_search(query="Azure SRE Agent connectors setup MCP")` |
| Incident platform integration | `microsoft_docs_search(query="Azure SRE Agent incident response PagerDuty ServiceNow")` |
| Permission management | `microsoft_docs_search(query="Azure SRE Agent permissions RBAC managed identity")` |
| Run modes (Review/Autonomous) | `microsoft_docs_fetch(url="https://sre.azure.com/docs/concepts/run-modes")` |
| Agent hooks (safety controls) | `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/agent-hooks")` |
| Scheduled tasks | `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/scheduled-tasks")` |
| Plugin marketplace | `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/plugin-marketplace")` |
| Deep investigation | `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/deep-investigation")` |
| Workflow automation | `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/workflow-automation")` |
| Full SRE Agent docs hub | `microsoft_docs_fetch(url="https://sre.azure.com/docs/")` |
| Sandbox lab (AKS) | Browse [github.com/matthansen0/azure-sre-agent-sandbox](https://github.com/matthansen0/azure-sre-agent-sandbox) |
| azd lab (Container Apps) | Browse [github.com/dm-chelupati/sre-agent-lab](https://github.com/dm-chelupati/sre-agent-lab) |
| Plugin development | Browse [github.com/Azure/sre-agent-plugins](https://github.com/Azure/sre-agent-plugins) |
| Code samples | `microsoft_code_sample_search(query="Azure SRE Agent", language="python")` |
