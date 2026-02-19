# Azure capacity management plugin

Claude Code plugin for Azure capacity and quota management, built for the azcapman repository.

## What's included

### Agent: azure-capacity-manager

Autonomous agent for operational analysis, planning, and engagement preparation. Triggers on capacity, quota, and reservation management tasks for SaaS ISVs.

**Example triggers:**
- Multi-subscription quota analysis
- Capacity reservation evaluation and cost modeling
- Quota group architecture design
- ISV engagement and workshop preparation
- AKS scaling failure diagnosis

### Skill: azure-capacity-management

Domain knowledge skill that activates when capacity-related topics arise in any conversation. Covers the full capacity supply chain: forecast, procure, allocate, and monitor.

## How references work

The skill uses symlinks to reference the full repository documentation without duplication:

```
skills/azure-capacity-management/references/
  docs -> ../../../docs           # All operational docs, billing, deployment patterns
  scripts -> ../../../scripts     # PowerShell/Python tools with READMEs
```

This keeps the skill in sync with the source documentation — no condensed duplicates to maintain.

## Integration

### Azure CLI
The agent uses `az` commands for live Azure operations — quota queries, CRG management, AKS operations, billing, and estate enumeration. No additional tooling required beyond an authenticated Azure CLI session.

### maenifold
When available, the agent uses maenifold skills for knowledge graph operations and context engineering across conversations.

### Microsoft Docs MCP
The agent can pull the latest Microsoft Learn content via `microsoft_docs_search` and `microsoft_docs_fetch` when repository documentation doesn't cover a specific scenario.

## Repository structure

```
.claude-plugin/plugin.json                        # Plugin manifest
agents/azure-capacity-manager.md                  # Agent definition
skills/azure-capacity-management/SKILL.md         # Skill definition
skills/azure-capacity-management/references/      # Symlinks to docs and scripts
```
