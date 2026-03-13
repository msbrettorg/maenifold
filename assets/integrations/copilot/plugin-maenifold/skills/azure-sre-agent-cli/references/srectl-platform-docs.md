# SRE Agent Platform Overview

AI-powered platform for Site Reliability Engineering. Automates incident response, monitoring, and operational tasks.

## Core Concepts

**Runtime Flow**: Trigger → Agent → Tools → Connectors

**Deployment Order**: Tools → Agents → Triggers (always deploy dependencies first)

| Concept | Description |
|---------|-------------|
| **Connectors** | Pre-authenticated endpoints (Kusto, ICM, Outlook, Teams, MCP) |
| **Tools** | Query/action definitions - System tools (platform) or Custom (KustoTool, LinkTool) |
| **Agents** | AI orchestration with instructions, tools, and optional handoffs |
| **Triggers** | Events that invoke agents (ICM incidents, scheduled tasks) |
| **Handoffs** | Agents delegating to other agents (use sparingly) |

## Key Principle: Kusto-Centric

Internal Microsoft teams diagnose Azure services via Kusto telemetry using (subscription, resourceGroup, resourceName) as keys. Do NOT mix with Azure Resource Graph or Az CLI - operators access their own telemetry, not customer resources directly.

## File Structure

```
your-project/
├── agents/
│   └── MyAgent/MyAgent.yaml
├── tools/
│   └── MyTool/MyTool.yaml
└── scheduledtasks/
```

---

# Deployment Workflow

## Design First, Deploy Second

### Phase 1: Design
1. Understand user requirements
2. List tools needed (system tools vs custom KustoTool/LinkTool)
3. Define agent with clear instructions
4. Get user confirmation

### Phase 2: Generate YAML
Generate YAML files for custom tools and agents.
**CRITICAL: Use `##param##` format for all parameter placeholders.**
**CRITICAL: Include `target: dictionary:args:string` for all parameters.**

### Phase 3: Deploy in Order

```
DEPLOYMENT ORDER (MANDATORY)
─────────────────────────────
1. TOOLS FIRST
   srectl apply-yaml --file tools/X/X.yaml

2. AGENTS SECOND
   srectl apply-yaml --file agents/Y/Y.yaml

3. SCHEDULED TASKS LAST (if any) - USE CLI COMMAND, NOT YAML
   srectl scheduledtask create --name "task-name" --agent "agent-name" --cron "0 8 * * *" --prompt "..."
```

## Why Order Matters

- Agents reference tools by name → tools must exist first
- Triggers reference agents → agents must exist first

# Kusto-Based Agent Workflow

## Key Principle: Kusto-Centric Diagnostics

Internal Microsoft teams diagnose Azure services via their own Kusto telemetry. Use (subscription, resourceGroup, resourceName) as keys to correlate data.

Do NOT use Azure Resource Graph or Az CLI - operators access telemetry, not customer resources directly.

## Building a Telemetry Agent

1. **Discover connectors**: `srectl tool show-connectors`
2. **Identify Kusto cluster/database** for your telemetry
3. **Create KustoTool** for each query pattern
4. **Create agent** that uses those tools

## CRITICAL: Parameter Format

- **ALWAYS use `##param##` format for placeholders** (NEVER `{{param}}`)
- **ALWAYS use `type: string` for all parameters**
- **ALWAYS use `target: dictionary:args:string` for all parameters**

## KustoTool Example

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: get-service-errors
spec:
  type: KustoTool
  connector: my-kusto-connector
  toolMode: Auto
  description: |-
    Purpose:
    Get error counts for a service within a time range

    Usage:
    Call with subscriptionId, resourceGroup, and hours parameters

    Output Format:
    Returns ErrorCount by hourly time bins
  database: TelemetryDB
  query: |-
    let _subscriptionId = '##subscriptionId##';
    let _resourceGroup = '##resourceGroup##';
    let _hours = toint('##hours##');
    ServiceLogs
    | where SubscriptionId == _subscriptionId
    | where ResourceGroup == _resourceGroup
    | where Timestamp > ago(_hours * 1h)
    | where Level == 'Error'
    | summarize ErrorCount=count() by bin(Timestamp, 1h)
  parameters:
    - name: subscriptionId
      type: string
      description: Azure subscription ID
      required: true
      target: dictionary:args:string
    - name: resourceGroup
      type: string
      description: Resource group name
      required: true
      target: dictionary:args:string
    - name: hours
      type: string
      description: Hours to look back
      required: true
      target: dictionary:args:string
```

# Pre-Deployment Checklist

Before deploying, verify:

## 1. Requirements Clear?
- [ ] User requirements understood
- [ ] Tools identified (system vs custom)
- [ ] Single agent sufficient? (avoid unnecessary subagents)

## 2. Connectors Verified?
- [ ] Ran `srectl tool show-connectors`
- [ ] KustoTool references existing Kusto connector
- [ ] Connector name matches exactly

## 3. Tools Ready? (CRITICAL FORMAT REQUIREMENTS)
- [ ] Custom tool YAML has connector + database (KustoTool)
- [ ] Custom tool YAML has template (LinkTool)
- [ ] All parameters have descriptions
- [ ] **All parameters use `type: string`**
- [ ] **All parameters have `target: dictionary:args:string`**
- [ ] **Query uses `##param##` format (NOT `{{param}}`)**

## 4. Agent Ready?
- [ ] Clear, specific instructions
- [ ] Tools reference only existing tools
- [ ] handoffDescription filled in
- [ ] handoffs: [] (empty unless truly needed)

## 5. Deploy Order?
- [ ] Tools FIRST
- [ ] Agents SECOND
- [ ] **Scheduled Tasks LAST - use CLI command, NOT YAML**

---

# SRE Agent Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    SRE Agent Platform                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  TRIGGER ──▶ AGENT ──▶ TOOLS ──▶ CONNECTORS                 │
│                                                             │
│  Examples:                                                  │
│  - ICM Incident    Agent with      KustoTool    Kusto       │
│  - Scheduled Task  instructions    LinkTool     ICM         │
│  - Manual                          SystemTools  Outlook     │
│                                                 Teams       │
│                                                 MCP         │
└─────────────────────────────────────────────────────────────┘
```

## Component Hierarchy

### Level 1: Connectors (Platform-Managed)
Pre-authenticated endpoints. Created by platform team via ARM.

| Type | Purpose |
|------|---------|
| Kusto | Query Kusto/ADX clusters |
| ICM | Incident management |
| Outlook | Send emails |
| Teams | Send notifications |
| MCP | External MCP servers |

### Level 2: Tools

**System Tools**: Pre-built platform tools (from SystemTools API)
- See `PublishedTools.json` for available tools
- Include charting, Azure operations, GitHub integration, etc.

**Custom Tools (BYO)**: User-defined via ExtendedAgentTool YAML
- KustoTool: Parameterized Kusto queries
- LinkTool: URL generation with parameters

### Level 3: Agents
LLM orchestration with tools and instructions.
- Model: gpt-5 (fixed)
- References tools by name
- Optional handoffs to other agents (use sparingly)

### Level 4: Triggers
- CronScheduledTask: Time-based
- ICM Incident Handler: Incident-driven

---

# srectl Command Reference

## Apply & Validate

```
srectl apply-yaml --file <path>    Apply YAML to platform
srectl validate --file <path>      Validate YAML without applying
```

## Discovery

```
srectl tool show-connectors        List available connectors (IMPORTANT!)
srectl tool list                   List all tools
srectl agent list                  List all agents
srectl scheduledtask list          List scheduled tasks
```

## Tool Operations

```
srectl tool get --name <name>      Get tool YAML
srectl tool delete --name <name>   Delete tool
srectl tool run --name <name>      Test tool execution
```

## Agent Operations

```
srectl agent get --name <name>     Get agent YAML
srectl agent delete --name <name>  Delete agent
srectl agent chat --name <name>    Interactive chat with agent
```

## Scheduled Task Operations

```
srectl scheduledtask get --name <name>    Get task config
srectl scheduledtask delete --name <name> Delete task
```

---

# Agent YAML Schema

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgent
metadata:
  name: my-agent
spec:
  instructions: |-
    Clear, specific instructions for the agent.
    What it should do, how it should respond.
  handoffDescription: 'One-line description for routing'
  handoffs: []
  tools:
    - tool-name-1
    - tool-name-2
  maxReflectionCount: 0
  customReflectionNote: ''
  commonPrompts: []
  enableVanillaMode: false
```

## Required Fields

| Field | Description |
|-------|-------------|
| api_version | `azuresre.ai/v2` |
| kind | `ExtendedAgent` |
| metadata.name | Unique kebab-case name |
| spec.instructions | Clear instructions for the LLM |
| spec.handoffDescription | Required. Short description |
| spec.handoffs | Required. Array (use `[]` if none) |

## Model

Fixed at `gpt-5`. Do not specify.

## Tools

Reference tools by name. Tool must exist before deploying agent.

```yaml
tools:
  - my-kusto-tool      # Custom tool you created
  - CheckAzureResource # System tool from platform
```

---

# Custom Tool Types (ExtendedAgentTool)

Custom tools let you define reusable queries and links.

## CRITICAL: Parameter Placeholder Format

**ALWAYS use `##paramName##` format for parameter placeholders in queries.**
- CORRECT: `'##subscriptionId##'`
- WRONG: `'{{subscriptionId}}'` or `'{subscriptionId}'`

**ALWAYS include `target: dictionary:args:string` for each parameter.**
**ALWAYS use `type: string` for all parameters.**

## KustoTool

Execute parameterized Kusto queries.

**Required Fields:**
- `connector`: Kusto connector name (from `srectl tool show-connectors`)
- `database`: Database name
- `description`: What the tool does

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: get-error-count
spec:
  type: KustoTool
  connector: my-kusto-connector
  toolMode: Auto
  description: Count errors for a resource in time range
  database: TelemetryDB
  query: |
    let _subscriptionId = '##subscriptionId##';
    let _resourceGroup = '##resourceGroup##';
    let _hours = toint('##hours##');
    ServiceLogs
    | where SubscriptionId == _subscriptionId
    | where ResourceGroup == _resourceGroup
    | where Timestamp > ago(_hours * 1h)
    | where Level == 'Error'
    | count
  parameters:
    - name: subscriptionId
      type: string
      description: Azure subscription ID
      required: true
      target: dictionary:args:string
    - name: resourceGroup
      type: string
      description: Resource group name
      required: true
      target: dictionary:args:string
    - name: hours
      type: string
      description: Hours to look back
      required: true
      target: dictionary:args:string
```

### Validating KustoTool Queries

**If Kusto MCP is available**, use it to validate queries before deployment:
1. Run `.show table TableName schema as json` to verify table/column names
2. Test query with hardcoded values first
3. Replace with `##param##` placeholders after validation
4. Run `srectl tool validate --file tool.yaml` before deployment

## LinkTool

Generate URLs with parameter substitution.

**Required Fields:**
- `template`: URL with `##param##` placeholders
- `description`: What the link is for

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: open-dashboard
spec:
  type: LinkTool
  toolMode: Auto
  description: Open Azure dashboard by ID
  template: https://portal.azure.com/#@/dashboard/##dashboardId##
  parameters:
    - name: dashboardId
      type: string
      description: Dashboard resource ID
      required: true
      target: dictionary:args:string
```

## Parameters

All parameters MUST have description, type: string, and target:

```yaml
parameters:
  - name: param-name
    type: string                      # ALWAYS use string
    description: What this parameter is for  # REQUIRED
    required: true                    # or false
    target: dictionary:args:string    # REQUIRED - always include this
```

---

# Triggers

Triggers invoke agents automatically.

## CronScheduledTask

**CRITICAL: DO NOT use YAML for scheduled tasks. ALWAYS use the CLI command instead.**

```powershell
srectl scheduledtask create \
  --name "daily-health-check" \
  --description "Daily health check task" \
  --cron "0 8 * * *" \
  --agent "health-check-agent" \
  --prompt "Run daily health check."
```

## Cron Expression Reference

```
┌───────────── minute (0-59)
│ ┌───────────── hour (0-23)
│ │ ┌───────────── day of month (1-31)
│ │ │ ┌───────────── month (1-12)
│ │ │ │ ┌───────────── day of week (0-6)
│ │ │ │ │
* * * * *
```

| Expression | Meaning |
|------------|---------|
| `0 * * * *` | Every hour |
| `0 8 * * *` | Daily at 8 AM |
| `0 */6 * * *` | Every 6 hours |
| `0 8 * * 1` | Monday at 8 AM |

## ICM Incident Handler

ICM-based triggers are configured separately via incident handler setup, not YAML.

---

# Handoffs (Use Sparingly)

Handoffs let agents delegate to other agents. Use only when truly needed.

## When to Use Handoffs

- Complex scenarios requiring specialized expertise
- Clear separation of concerns (e.g., incident triage vs. remediation)

## When NOT to Use Handoffs

- Simple single-purpose agents (most cases)
- When one agent with multiple tools suffices
- To artificially split functionality

## If You Need Handoffs

1. Deploy child agents first (they have no handoffs)
2. Deploy parent agent last (references children)

```yaml
# Parent agent references children
spec:
  handoffs:
    - specialist-agent-1
    - specialist-agent-2
```

Keep it simple. One well-designed agent is usually better than many small ones.

---

# Building Agents

## Simple Agent (Recommended)

Most use cases need just one agent with appropriate tools:

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgent
metadata:
  name: service-health-agent
spec:
  instructions: |-
    You help investigate service health issues.

    Use your Kusto tools to query telemetry.
    Look for errors, latency spikes, and anomalies.
    Correlate using subscriptionId, resourceGroup, resourceName.
  handoffDescription: 'Investigates service health using telemetry'
  handoffs: []
  tools:
    - get-service-errors
    - get-latency-metrics
    - get-request-volume
  maxReflectionCount: 0
  customReflectionNote: ''
  commonPrompts: []
  enableVanillaMode: false
```

## Deployment

```powershell
# 1. Deploy custom tools first
srectl apply-yaml --file tools/get-service-errors.yaml
srectl apply-yaml --file tools/get-latency-metrics.yaml

# 2. Deploy agent second
srectl apply-yaml --file agents/service-health-agent.yaml
```

---

# Scheduled Tasks

Run agents on a schedule.

## CRITICAL: Always Use CLI Command - DO NOT Use YAML

**ALWAYS use `srectl scheduledtask create` command. DO NOT use YAML files for scheduled tasks.**

## Creating a Scheduled Task

```powershell
srectl scheduledtask create \
  --name "daily-token-consumption-report" \
  --description "Daily scheduled task that monitors SRE Agent token consumption" \
  --cron "0 8 * * *" \
  --agent "token-consumption-monitor" \
  --prompt "Generate a daily SRE Agent token consumption report. Query the last 7 days of token consumption data to show trends. Identify the top consuming agents and subscriptions. Compare today's consumption with the 7-day average. Highlight any significant changes or anomalies."
```

## Deployment Order

```powershell
# 1. Tools (if any custom tools)
srectl apply-yaml --file tools/my-tool.yaml

# 2. Agent
srectl apply-yaml --file agents/health-check-agent.yaml

# 3. Scheduled task LAST - ALWAYS use CLI command
srectl scheduledtask create --name "daily-health-check" --agent "health-check-agent" --cron "0 8 * * *" --prompt "Run the daily health check."
```

## Management

```powershell
srectl scheduledtask list
srectl scheduledtask get --name daily-health-check
srectl scheduledtask delete --name daily-health-check
```

---

# System Tools

The platform provides pre-built system tools for common operations.

## Discovering System Tools

System tools are available via the platform API. Key categories include:

- **Azure Operations**: CheckAzureResource, RestartAppService, etc.
- **ICM**: GetIncidentInfo, PostDiscussionEntry, etc.
- **GitHub**: CreateIssue, FetchIssues, etc.
- **Charting**: GenerateBarChart, GeneratePieChart, etc.
- **DGrep**: Query distributed logs

## Using System Tools

Reference system tools by name in your agent's tools list:

```yaml
tools:
  - CheckAzureResource    # System tool
  - my-custom-kusto-tool  # Your custom KustoTool
```

System tools don't need YAML - they're provided by the platform.
Custom tools (KustoTool, LinkTool) need ExtendedAgentTool YAML.

---

# YAML Schema Reference (V2)

## API Version

All YAML uses: `api_version: azuresre.ai/v2`

## Agent Schema

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgent
metadata:
  name: agent-name                    # kebab-case
spec:
  instructions: |-                    # Multi-line instructions
    Clear instructions for the agent.
  handoffDescription: 'Short desc'   # REQUIRED
  handoffs: []                        # REQUIRED (can be empty)
  tools:
    - tool-name-1
    - tool-name-2
  maxReflectionCount: 0
  customReflectionNote: ''
  commonPrompts: []
  enableVanillaMode: false
```

## KustoTool Schema

**CRITICAL: Always use `##param##` format for placeholders, NEVER use `{{param}}`**
**CRITICAL: Always use `type: string` and `target: dictionary:args:string` for all parameters**

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: tool-name                     # kebab-case
spec:
  type: KustoTool
  connector: connector-name           # REQUIRED - from show-connectors
  toolMode: Auto                      # Auto or Manual (default: Auto)
  description: |-                     # Use |- for multi-line descriptions
    Purpose:
    What this tool does and when to use it

    Usage:
    How to call this tool with parameters

    Output Format:
    What columns/data the tool returns
  database: DatabaseName              # REQUIRED
  query: |-
    let _startTime = todatetime('##startTime##');
    let _endTime = todatetime('##endTime##');
    let _resourceId = '##resourceId##';
    Table
    | where Timestamp between (_startTime .. _endTime)
    | where ResourceId == _resourceId
  parameters:
    - name: startTime
      type: string
      description: Start time for the query (e.g., 2024-01-01T00:00:00Z)
      required: true
      target: dictionary:args:string
    - name: endTime
      type: string
      description: End time for the query
      required: true
      target: dictionary:args:string
    - name: resourceId
      type: string
      description: Azure resource ID
      required: true
      target: dictionary:args:string
```

### Tool Mode

- `Auto`: Tool is automatically available to the agent (default)
- `Manual`: Tool requires explicit user approval before execution

### Query Schema Validation

When an agent has KustoTools, always validate the query schema:
1. Ensure parameter placeholders use `##param##` format (NEVER `{{param}}`)
2. Ensure all parameters have `type: string` and `target: dictionary:args:string`
3. Verify database and table names are correct
4. Test queries with sample data before deployment
5. Use `srectl tool validate --file tool.yaml` to validate syntax

**If Kusto MCP is available**: Use it to validate queries before deployment:
- Run `.show table TableName schema as json` to verify table/column names
- Test query with hardcoded values first, then replace with `##param##` placeholders
- Verify the query returns expected results with sample data

**For client agents with tools**: Before deployment, validate that:
- All referenced tools exist and are accessible
- Query parameters use `##param##` format, NOT `{{param}}`
- All parameters have `target: dictionary:args:string`
- Database permissions are correctly configured for the connector
- Run `srectl agent validate --name agent-name` to check tool bindings

## LinkTool Schema

**CRITICAL: Use `##param##` format for placeholders in templates**

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: tool-name                     # kebab-case
spec:
  type: LinkTool
  toolMode: Auto                      # Auto or Manual (default: Auto)
  description: What this link is for  # REQUIRED
  template: https://url/##param##     # REQUIRED - use ##param## format
  parameters:
    - name: param
      type: string
      description: Parameter purpose  # REQUIRED
      required: true
      target: dictionary:args:string  # REQUIRED
```

## Scheduled Task Schema

**CRITICAL: DO NOT use YAML for scheduled tasks. ALWAYS use the CLI command instead.**

Use `srectl scheduledtask create` command:
```powershell
srectl scheduledtask create \
  --name "daily-report" \
  --description "Daily scheduled task description" \
  --cron "0 8 * * *" \
  --agent "target-agent-name" \
  --prompt "What to ask the agent"
```

The YAML schema below is for reference only - DO NOT USE IT:
```yaml
# DO NOT USE YAML FOR SCHEDULED TASKS - USE CLI COMMAND ABOVE
api_version: azuresre.ai/v2
kind: CronScheduledTask
metadata:
  name: task-name
spec:
  agentName: target-agent             # Agent to invoke
  schedule: '0 8 * * *'               # Cron expression
  prompt: |-
    What to ask the agent.
  enabled: true
```

## Naming

- Use kebab-case: `my-agent-name`
- Keep names descriptive but concise

---

# Connectors

Connectors are pre-authenticated endpoints for data sources.

## Available Connector Types

| Type | Purpose |
|------|---------|
| Kusto | Query Kusto/ADX clusters |
| ICM | Incident management |
| Outlook | Send emails |
| Teams | Send notifications |
| MCP | Connect to external MCP servers |

## Workflow: Check Before Creating

1. **List existing connectors**: Use `list_connectors` tool
2. **Reuse if exists**: If your Kusto cluster already has a connector, use it
3. **Create if needed**: Use `create_kusto_connector` tool

## Creating a Kusto Connector

The `create_kusto_connector` tool will:
1. Check if connector already exists (reuse it)
2. If not found, provide Az CLI/PowerShell commands for manual creation

### Connector Properties

| Property | Description |
|----------|-------------|
| name | Unique connector name (kebab-case) |
| dataConnectorType | `Kusto`, `ICM`, `Outlook`, `Teams`, `MCP` |
| dataSource | Cluster URL for Kusto |
| identity | `system` (uses managed identity) |

## Using Connectors in KustoTool

```yaml
spec:
  type: KustoTool
  connector: my-kusto-connector   # Exact name from list_connectors
  database: TelemetryDB
```

## Permission Requirements

- **List connectors**: Reader access
- **Create connectors**: Contributor access on the Agent resource

The tool provides Az CLI and PowerShell commands for users to create connectors themselves.

---

# MCP Server Configuration

## VS Code

Add to `.vscode/settings.json`:

```json
{
  "mcp.servers": {
    "srectl": {
      "command": "srectl",
      "args": ["mcp", "start"]
    }
  }
}
```

## Claude Desktop

Add to config:

```json
{
  "mcpServers": {
    "srectl": {
      "command": "srectl",
      "args": ["mcp", "start"]
    }
  }
}
```

## MCP Tools Available

| Tool | Purpose |
|------|---------|
| plan_agent_architecture | Design solution (call first) |
| generate_and_validate_yaml | Generate agent/tool YAML |
| get_documentation | Get platform docs |
| list_stable_tool_types | List BYO tool types |

---

# Quick Start

## Workflow Overview

**IMPORTANT: Always follow this sequence:**
1. Plan architecture → **GET USER CONFIRMATION**
2. Check/create connectors
3. Generate YAML → **GET USER CONFIRMATION**
4. Deploy and test

---

## Step 1: Plan Architecture

Call `plan_agent_architecture` with user requirements.

**CRITICAL:** After receiving the plan:
- Display the mermaid diagram to the user
- Ask: "Does this architecture look correct?"
- **DO NOT proceed until user confirms**

---

## Step 2: Check Existing Connectors

Use `list_connectors` tool to see what's already available.

---

## Step 3: Create Connector if Needed

If no Kusto connector exists for your cluster:
```
Use create_kusto_connector tool:
- name: my-kusto-connector
- clusterUrl: https://mycluster.eastus2.kusto.windows.net
```

If permission denied (403), the tool provides Az CLI commands to run manually.

---

## Step 4: Create a KustoTool

**CRITICAL: Always use `##param##` format for placeholders. Always include `target: dictionary:args:string`.**

```yaml
# tools/get-errors.yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: get-errors
spec:
  type: KustoTool
  connector: my-kusto-connector      # From step 2 or 3
  toolMode: Auto                     # Auto or Manual
  description: |-
    Purpose:
    Get errors for a resource within a time range

    Usage:
    Call with subscriptionId and hours parameters

    Output Format:
    Returns error log entries (limited to 100)
  database: TelemetryDB
  query: |-
    let _subscriptionId = '##subscriptionId##';
    let _hours = toint('##hours##');
    ServiceLogs
    | where SubscriptionId == _subscriptionId
    | where Timestamp > ago(_hours * 1h)
    | where Level == 'Error'
    | take 100
  parameters:
    - name: subscriptionId
      type: string
      description: Azure subscription ID
      required: true
      target: dictionary:args:string
    - name: hours
      type: string
      description: Hours to look back
      required: true
      target: dictionary:args:string
```

---

## Step 5: Deploy the Tool

```powershell
srectl apply-yaml --file tools/get-errors.yaml
```

---

## Step 6: Create an Agent

```yaml
# agents/error-analyst.yaml
api_version: azuresre.ai/v2
kind: ExtendedAgent
metadata:
  name: error-analyst
spec:
  instructions: |-
    You analyze service errors from telemetry.
    Use get-errors to query error logs.
    Look for patterns and suggest root causes.
  handoffDescription: 'Analyzes service errors'
  handoffs: []
  tools:
    - get-errors
  maxReflectionCount: 0
  customReflectionNote: ''
  commonPrompts: []
  enableVanillaMode: false
```

---

## Step 7: Deploy and Test

```powershell
srectl apply-yaml --file agents/error-analyst.yaml
srectl agent chat --name error-analyst
```
