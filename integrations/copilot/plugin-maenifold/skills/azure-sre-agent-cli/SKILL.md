---
name: azure-sre-agent-cli
description: |
  Use when the user asks about Azure SRE Agent, srectl CLI, or building agents, skills,
  tools, scheduled tasks, or incident handlers for the SRE Agent platform. Covers:
  installing and configuring srectl, creating and deploying agents via YAML, building
  KustoTool and LinkTool definitions, managing skills and knowledge docs, configuring
  hooks for safety and governance, setting up scheduled tasks, working with MCP server
  integration, and managing profiles for multiple SRE Agent instances. Also covers the
  SRE Agent architecture (trigger → agent → tools → connectors), deployment order
  (tools first, agents second, triggers last), and YAML schema (api_version azuresre.ai/v2).
---

# Azure SRE Agent and srectl CLI

[Azure SRE Agent](https://learn.microsoft.com/en-us/azure/sre-agent/overview) is an AI-powered reliability platform that automates incident response, monitoring, and operational tasks. The `srectl` CLI is the command-line interface for managing SRE Agent instances—creating agents, skills, tools, and scheduled tasks via YAML configurations.

## Installation

```bash
# Install the Azure Artifacts credential provider (one-time)
curl -L https://raw.githubusercontent.com/microsoft/artifacts-credprovider/master/helpers/installcredprovider.sh | bash

# Install srectl (.NET 9.0 SDK required)
dotnet tool install sreagent.cli --global \
  --add-source https://pkgs.dev.azure.com/msazure/One/_packaging/SREAgentCli/nuget/v3/index.json \
  --interactive

# Verify
srectl --version
```

**Prerequisites:** .NET 9.0 SDK, Azure CLI (`az login` for authentication).

## Key concepts

### Architecture: trigger → agent → tools → connectors

The runtime flow is: a **trigger** (ICM incident, scheduled task, or manual chat) invokes an **agent**, which uses **tools** (KustoTool, LinkTool, or system tools) that connect to **connectors** (pre-authenticated endpoints for Kusto, ICM, Outlook, Teams, MCP).

### Deployment order

Always deploy dependencies first: **tools → agents → scheduled tasks**. Agents reference tools by name, so tools must exist before agents. Scheduled tasks reference agents, so agents must exist before tasks.

### YAML schema (V2)

All YAML uses `api_version: azuresre.ai/v2`. Two `kind` values: `ExtendedAgent` for agents, `ExtendedAgentTool` for tools. Use `##param##` format for parameter placeholders in queries and templates—never `{{param}}`.

### Workspace structure

```
your-project/
├── agents/
│   └── MyAgent/MyAgent.yaml
├── tools/
│   └── MyTool/MyTool.yaml
├── skills/
│   └── my-skill/SKILL.md
└── scheduledtasks/
```

### Profiles

Profiles store connection settings for different SRE Agent instances. Use `srectl profile create` to add instances and `srectl profile set` to switch between them.

## Command reference

### Initialization and status

| Command | Purpose |
|---------|---------|
| `srectl init --resource-url <url>` | Connect to an SRE Agent instance |
| `srectl status` | Show workspace health |
| `srectl sync` | Pull remote configs to local workspace |
| `srectl profile list` | List connection profiles |
| `srectl profile create --name <n> --url <url>` | Create a new profile |
| `srectl profile set --name <n>` | Switch active profile |

### Agent lifecycle

| Command | Purpose |
|---------|---------|
| `srectl agent create --name <n> --instructions "..."` | Create agent YAML |
| `srectl agent create --name <n> --smart --instructions "..."` | Create with AI assistance |
| `srectl agent validate --name <n>` | Validate agent YAML locally |
| `srectl agent validate --all --check-tools` | Validate all agents with tool checks |
| `srectl agent apply --name <n>` | Deploy agent to server |
| `srectl agent apply --name <n> --dry-run` | Preview deployment |
| `srectl agent test --name <n> --message "..."` | Test agent interactively |
| `srectl agent diff --name <n>` | Compare local vs remote |
| `srectl agent list` | List remote agents |
| `srectl agent delete --name <n>` | Delete remote agent |

### Skill lifecycle

| Command | Purpose |
|---------|---------|
| `srectl skill create --name <n>` | Create skill directory with templates |
| `srectl skill apply --name <n>` | Upload skill to server |
| `srectl skill apply --name <n> --dry-run` | Preview upload |
| `srectl skill list` | List remote skills |
| `srectl skill sync --name <n>` | Download skill from server |
| `srectl skill sync --all` | Download all skills |
| `srectl skill convert --agent-name <n>` | Convert agent to skill |
| `srectl skill delete --name <n>` | Delete remote skill |

### Tool lifecycle

| Command | Purpose |
|---------|---------|
| `srectl tool create` | Create tool YAML |
| `srectl tool validate --name <n>` | Validate tool YAML |
| `srectl tool apply --name <n>` | Deploy tool to server |
| `srectl tool show-types` | List available tool types (KustoTool, LinkTool) |
| `srectl tool show-connectors` | List configured connectors |
| `srectl tool list` | List remote tools |
| `srectl tool delete --name <n>` | Delete remote tool |

### Apply YAML (kubectl-style)

| Command | Purpose |
|---------|---------|
| `srectl apply-yaml --file <path>` | Apply single YAML file |
| `srectl apply-yaml --file manifests/all.yaml` | Apply multi-document YAML |

### Knowledge documents

| Command | Purpose |
|---------|---------|
| `srectl doc upload --file <path>` | Upload a document |
| `srectl doc upload --file ./docs` | Upload entire folder |
| `srectl doc search --query "..."` | Search knowledge base |
| `srectl doc get` | List uploaded documents |
| `srectl doc delete --name <n>` | Delete a document |
| `srectl doc reindex` | Reindex all documents |

### Scheduled tasks

| Command | Purpose |
|---------|---------|
| `srectl scheduledtask create --name "..." --cron "..." --prompt "..." --agent "..."` | Create scheduled task |
| `srectl scheduledtask list` | List tasks |
| `srectl scheduledtask pause --id <id>` | Pause task |
| `srectl scheduledtask resume --id <id>` | Resume task |
| `srectl scheduledtask delete --id <id>` | Delete task |

### Hooks (safety and governance)

| Command | Purpose |
|---------|---------|
| `srectl hook list` | List hooks |
| `srectl hook apply --file <path>` | Create/update hook |
| `srectl hook activate --thread-id <id> --name <n>` | Activate for thread |
| `srectl hook deactivate --thread-id <id> --name <n>` | Deactivate for thread |

### MCP server

| Command | Purpose |
|---------|---------|
| `srectl mcp start` | Start MCP server (stdio transport) |
| `srectl mcp info` | Show platform documentation |
| `srectl mcp info --topic <topic>` | Show specific topic |
| `srectl mcp info --all` | Show all documentation |

### Interactive and chat

| Command | Purpose |
|---------|---------|
| `srectl chat` | Start interactive chat with SRE Agent |
| `srectl interactive` | Start guided mode |

## Agent YAML schema

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgent
metadata:
  name: my-agent                      # kebab-case
spec:
  instructions: |-                    # Multi-line agent instructions
    You are a specialist that...
  handoffDescription: 'Short description for routing'
  handoffs: []                        # Other agents this can delegate to
  tools:
    - tool-name-1
    - tool-name-2
  maxReflectionCount: 0
  customReflectionNote: ''
  commonPrompts: []
  enableVanillaMode: false
```

## KustoTool YAML schema

**Use `##param##` for placeholders. Use `type: string` and `target: dictionary:args:string` for all parameters.**

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: get-service-errors
spec:
  type: KustoTool
  connector: my-kusto-connector       # From srectl tool show-connectors
  toolMode: Auto
  description: |-
    Purpose: Get error counts for a service
    Usage: Call with subscriptionId and hours
    Output Format: ErrorCount by hourly bins
  database: TelemetryDB
  query: |-
    let _sub = '##subscriptionId##';
    let _hours = toint('##hours##');
    ServiceLogs
    | where SubscriptionId == _sub
    | where Timestamp > ago(_hours * 1h)
    | where Level == 'Error'
    | summarize ErrorCount=count() by bin(Timestamp, 1h)
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

## LinkTool YAML schema

```yaml
api_version: azuresre.ai/v2
kind: ExtendedAgentTool
metadata:
  name: open-dashboard
spec:
  type: LinkTool
  toolMode: Auto
  description: Open the Grafana dashboard for a resource
  template: https://grafana.example.com/d/##dashboardId##?var-resource=##resourceName##
  parameters:
    - name: dashboardId
      type: string
      description: Grafana dashboard ID
      required: true
      target: dictionary:args:string
    - name: resourceName
      type: string
      description: Resource name to filter
      required: true
      target: dictionary:args:string
```

## Deployment workflow

1. Check connectors: `srectl tool show-connectors`
2. Create tools: `srectl apply-yaml --file tools/my-tool.yaml`
3. Create agent: `srectl apply-yaml --file agents/my-agent.yaml`
4. Test: `srectl agent test --name my-agent --message "check system health"`
5. Schedule (if needed): `srectl scheduledtask create --name "daily-check" --cron "0 9 * * *" --agent "my-agent" --prompt "Run daily health check"`

## MCP server configuration

### Copilot CLI

Add to `~/.copilot/mcp-config.json`:

```json
{
  "mcpServers": {
    "srectl": {
      "type": "stdio",
      "command": "/Users/brett/.dotnet/tools/srectl",
      "args": ["mcp", "start"],
      "env": {
        "DOTNET_ROOT": "/Users/brett/.dotnet"
      }
    }
  }
}
```

### VS Code

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

## Common patterns

### Deploy a capacity management skill

```bash
srectl init --resource-url https://my-agent.1abc.eastus2.azuresre.ai
srectl skill apply --name azure-capacity-management
srectl apply-yaml --file sre-agent/subagent/capacity-manager.yaml
srectl doc upload --file docs/operations/
```

### Create a scheduled quota check

```bash
srectl scheduledtask create \
  --name "daily-quota-check" \
  --cron "0 9 * * *" \
  --agent "capacity-manager" \
  --prompt "Check quota utilization across all production subscriptions and alert if any VM family exceeds 80%"
```

### Validate before deploying

```bash
srectl agent validate --all --check-tools
srectl tool validate --all
```

## Learn more

Use `srectl mcp info --topic <topic>` for detailed documentation on any topic:

| Topic | Description |
|-------|-------------|
| `overview` | Platform overview |
| `architecture` | Trigger → agent → tool flow |
| `cli` | CLI command structure |
| `agents` | Agent configuration |
| `tools` | Tool configuration |
| `triggers` | Trigger types |
| `subagents` | Subagent and handoff concepts |
| `scheduled-tasks` | Scheduled task configuration |
| `quickstart` | Getting started |

**Source**: [Azure SRE Agent documentation](https://learn.microsoft.com/en-us/azure/sre-agent/)
