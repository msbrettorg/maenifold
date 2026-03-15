# Custom Agent (Subagent) YAML Patterns

Common patterns for authoring Azure SRE Agent custom agents.

## API Format

SRE Agent configs use the `azuresre.ai/v1` API format, deployable via REST API or the portal UI:

```yaml
api_version: azuresre.ai/v1
kind: AgentConfiguration
spec:
  name: <agent-name>
  system_prompt: |
    <instructions>
  handoff_description: <one-line summary for orchestrator>
  agent_type: Autonomous | Review
  tools:
    - <tool-name>
  mcp_tools:
    - <connection-id>/*
  allowed_skills:
    - <skill-name>
```

## Real-World Example: Incident Handler (from dm-chelupati/sre-agent-lab)

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

## Real-World Example: Code Analyzer with GitHub MCP

```yaml
api_version: azuresre.ai/v1
kind: AgentConfiguration
spec:
  name: code-analyzer
  system_prompt: |
    You are an expert in diagnosing, finding root cause, and mitigating issues
    in applications. When triggered:

    1. Search the knowledge base for the relevant runbook and execute ALL
       diagnostic steps — query logs, check metrics, collect evidence.
    2. Use source code from the connected GitHub repo to correlate log errors
       to specific code paths. Provide file:line references to the root cause.
    3. Combine both log/metric evidence AND source code analysis into a full
       root cause analysis.

    Create a GitHub issue with detailed findings including log evidence,
    metrics, code snippets, suggested fixes, and remediation actions.

    Always search memory for similar past incidents first.
    Use ExecutePythonCode to plot metrics charts when presenting evidence.
  handoff_description: Deep root cause analysis using runbooks + source code, creates GitHub issues
  agent_type: Autonomous
  tools:
    - SearchMemory
    - RunAzCliReadCommands
    - RunAzCliWriteCommands
    - GetAzCliHelp
    - QueryLogAnalyticsByWorkspaceId
    - QueryAppInsightsByResourceId
    - ExecutePythonCode
  mcp_tools:
    - github-mcp/*
```

## Available Built-in Tool Names

| Tool | Purpose |
|------|---------|
| `SearchMemory` | Search knowledge base and past incidents |
| `RunAzCliReadCommands` | Execute read-only Azure CLI commands |
| `RunAzCliWriteCommands` | Execute write Azure CLI commands (requires approval in Review mode) |
| `GetAzCliHelp` | Get Azure CLI help text |
| `QueryLogAnalyticsByWorkspaceId` | Run KQL queries against Log Analytics |
| `QueryAppInsightsByResourceId` | Query Application Insights |
| `ExecutePythonCode` | Run Python in sandboxed interpreter |
| `SendNotificationToTeams` | Post to Teams (requires Teams connector) |
| `SendEmailViaOutlook` | Send email (requires Outlook connector) |

## Task Specialist Pattern

Focused on specific tasks across services:

```yaml
name: cost_optimizer
system_prompt: |
  You are a cost optimization specialist. Analyze Azure spending patterns
  and recommend savings opportunities.
  
  Focus areas:
  - Idle or underutilized resources
  - Reserved instance coverage gaps
  - Storage tier optimization
  - Oversized VM SKUs
  
  Always provide estimated monthly savings with each recommendation.
  Present findings in a structured table format.
handoff_description: Analyzes Azure costs and recommends optimization opportunities
tools:
  - RunAzCliReadCommands
```

## Workflow Executor Pattern

Multi-step procedures:

```yaml
name: incident_triage
system_prompt: |
  You are an incident triage specialist. When an incident arrives:
  
  1. CLASSIFY: Determine severity (P1-P4) based on blast radius and customer impact
  2. SCOPE: Identify all affected Azure resources and dependencies
  3. INVESTIGATE: Query logs, metrics, and traces for the last 30 minutes
  4. CORRELATE: Check recent deployments, config changes, and similar past incidents
  5. RECOMMEND: Suggest immediate mitigation steps
  6. NOTIFY: Send summary to the appropriate Teams channel
  
  For P1 incidents, always check for:
  - Recent deployments in the last 2 hours
  - Cross-region impact
  - Data plane vs control plane issues
handoff_description: Classifies, scopes, and triages incoming incidents
tools:
  - RunAzCliReadCommands
  - execute_kusto_query
connectors:
  - pagerduty
allowed_skills:
  - incident-runbook
```

## With MCP Tools (Wildcard)

```yaml
name: observability_expert
system_prompt: |
  You are an observability expert with access to both Azure Monitor and Datadog.
  Correlate signals across both platforms to find root causes.
handoff_description: Cross-platform observability analysis using Azure Monitor and Datadog
tools:
  - RunAzCliReadCommands
mcp_tools:
  - datadog-mcp/*
  - github_search_code
```

## With Maenifold Knowledge Graph

```yaml
name: knowledge_engineer
system_prompt: |
  You are a knowledge graph specialist using maenifold.
  
  When asked to research a topic:
  1. SearchMemories to find existing knowledge
  2. BuildContext to understand concept relationships
  3. FindSimilarConcepts for semantic discovery
  4. Write findings with WriteMemory using [[WikiLinks]]
  
  When asked to organize knowledge:
  1. AnalyzeConceptCorruption to find duplicates
  2. RepairConcepts to consolidate variants
  3. Sync to update the graph
  
  Always ground answers in the knowledge graph.
  Always use [[WikiLinks]] in written content.
handoff_description: Manages persistent knowledge graph, researches topics, writes structured memory
mcp_tools:
  - maenifold-mcp/*
```

## Notification Router

```yaml
name: notification_router
system_prompt: |
  You are a notification specialist. Route investigation findings
  to the appropriate channels based on severity and team ownership.
  
  Routing rules:
  - P1: Teams #incidents + email to on-call
  - P2: Teams #incidents
  - P3-P4: Teams #operations
  
  Always include: affected resource, summary, recommended action, evidence link.
handoff_description: Routes investigation findings to Teams and email based on severity
tools:
  - SendNotificationToTeams
  - SendEmailViaOutlook
```

## Run Modes

Each custom agent operates in a mode set per response plan or scheduled task:

| Mode | Description | When to Use |
|------|-------------|-------------|
| **Review** | Proposes actions, waits for approval | Cost changes, write operations, risky mitigations |
| **Autonomous** | Acts without human approval | Well-tested automation, read-only operations |

Modes are configured per response plan or scheduled task, NOT in the YAML.

## Connector YAML Format

MCP connectors also use the `azuresre.ai/v1` API format:

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

## Creating via Portal

1. **Builder → Agent Canvas → Create → Custom Agent**
2. Fill: Name, Instructions
3. Optional: Choose Skills, Choose Tools, Manage Hooks
4. **Create**
5. Test in **Test playground** (split-screen: config left, chat right)

## Tips

- Keep instructions focused — one domain per agent
- Use `allowed_skills` to combine with existing skill procedures
- Use `mcp_tools` wildcards for trusted MCP servers
- Test in Playground before production use
- Set Review mode for anything that writes or costs money
