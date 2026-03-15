# Skill YAML Patterns

Common patterns for authoring Azure SRE Agent skills.

## Basic Skill (Knowledge Only)

```yaml
name: deployment-runbook
description: Standard deployment procedures for our microservices platform
```

```markdown
---
name: deployment-runbook
description: Standard deployment procedures for our microservices platform
---

## When to use this skill
Use when handling deployment-related issues, rollbacks, or deployment validation.

## Pre-deployment checklist
1. Verify CI pipeline passed on the target commit
2. Check for active incidents on dependent services
3. Confirm deployment window with on-call

## Rollback procedure
1. Identify the last known good deployment version
2. Run `az containerapp revision activate` for the previous revision
3. Verify health endpoints return 200
4. Notify #deployments channel
```

## Skill with Azure CLI Tools

```yaml
name: aks-troubleshooting
description: Diagnose AKS cluster issues including pod failures, node pressure, and networking
tools:
  - RunAzCliReadCommands
  - RunAzCliWriteCommands
```

```markdown
---
name: aks-troubleshooting
description: Diagnose AKS cluster issues including pod failures, node pressure, and networking
tools:
  - RunAzCliReadCommands
  - RunAzCliWriteCommands
---

## Steps
1. Get cluster health: `az aks show --resource-group <rg> --name <cluster>`
2. Check node status: `kubectl get nodes -o wide`
3. Find failing pods: `kubectl get pods --all-namespaces --field-selector=status.phase!=Running`
4. Check events: `kubectl get events --sort-by=.metadata.creationTimestamp | tail -20`
5. If node pressure detected, check resource usage: `kubectl top nodes`

## Common issues
- **ImagePullBackOff**: Check ACR credentials and image tag
- **CrashLoopBackOff**: Check pod logs with `kubectl logs <pod> --previous`
- **Pending pods**: Check resource quotas and node capacity
```

## Skill with Kusto Tools

```yaml
name: app-service-diagnostics
description: Investigate App Service performance and availability issues using log queries
tools:
  - execute_kusto_query
  - RunAzCliReadCommands
```

```markdown
---
name: app-service-diagnostics
description: Investigate App Service performance and availability issues using log queries
tools:
  - execute_kusto_query
  - RunAzCliReadCommands
---

## Investigation flow
1. Query Application Insights for error rate:
   ```kql
   requests
   | where timestamp > ago(1h)
   | summarize errorRate = countif(success == false) * 100.0 / count() by bin(timestamp, 5m)
   | order by timestamp desc
   ```
2. Check for dependency failures:
   ```kql
   dependencies
   | where timestamp > ago(1h) and success == false
   | summarize count() by target, type
   | order by count_ desc
   ```
3. Check App Service metrics via Azure CLI
4. Correlate with recent deployments
```

## Skill with MCP Tools

```yaml
name: datadog-correlation
description: Correlate Azure issues with Datadog metrics and traces
tools:
  - RunAzCliReadCommands
mcp_tools:
  - datadog-mcp/*
```

## Skill with Supporting Files

When a skill needs reference data beyond SKILL.md:

```
my-skill/
├── SKILL.md
├── known-good-baselines.json
├── query-templates/
│   ├── cpu-analysis.kql
│   └── memory-analysis.kql
└── runbook-checklist.md
```

Add files via **Builder → Skills → [skill] → Files → new file/upload folder**.

## Tips

- **Be specific in description** — agent reads it to decide relevance
- **Number your steps** — gives agent a clear playbook
- **Specify expected output** — tell agent what format results should take
- **Include KQL templates** — reduces agent hallucination on queries
- **Max 5 concurrent skills** — prioritize the most commonly needed
