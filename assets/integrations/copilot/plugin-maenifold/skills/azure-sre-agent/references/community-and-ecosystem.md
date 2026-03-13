# Community Resources & Ecosystem

Azure SRE Agent has a growing community of open-source repos, plugins, and sandbox environments.

## Official Repositories

| Repository | What You Get |
|-----------|-------------|
| [microsoft/sre-agent](https://github.com/microsoft/sre-agent) | Official community hub. Bug reports, feature requests, issue templates with Thread ID guidance |
| [Azure/sre-agent-plugins](https://github.com/Azure/sre-agent-plugins) | Official plugin marketplace. AWS and Azure Managed Grafana plugins, plugin development templates |

## Sandbox & Lab Environments

### matthansen0/azure-sre-agent-sandbox (⭐ 36)

Full AKS-based demo lab with **10 breakable scenarios** and automated Bicep deployment.

**Scenarios available:**

| Scenario | Description | What SRE Agent Diagnoses |
|----------|-------------|-------------------------|
| OOMKilled | Memory limit too low | Memory exhaustion, limit recommendations |
| CrashLoop | App exits immediately | Exit codes, log analysis |
| ImagePullBackOff | Invalid image reference | Registry/image troubleshooting |
| HighCPU | Resource exhaustion | Performance analysis |
| PendingPods | Insufficient cluster resources | Scheduling analysis |
| ProbeFailure | Failing health checks | Probe configuration |
| NetworkBlock | NetworkPolicy blocking traffic | Connectivity analysis |
| MissingConfig | Non-existent ConfigMap | Configuration troubleshooting |
| MongoDBDown | Database offline, cascading failure | Dependency tracing, root cause |
| ServiceMismatch | Wrong Service selector, silent failure | Endpoint/selector analysis |

**Deploy:** `.\scripts\deploy.ps1 -Location eastus2 -Yes` (~15-25 min)

**Example prompts for SRE Agent:**
- "Why are pods crashing in the pets namespace?"
- "What's causing high CPU usage?"
- "Diagnose the CrashLoopBackOff error"

### dm-chelupati/sre-agent-lab (⭐ 2)

Single `azd up` deployment with pre-configured subagents, knowledge base, and response plans.

**Deploys:** SRE Agent (`Microsoft.App/agents`), Container App (Grubify), Log Analytics, App Insights, Alert Rules, Managed Identity.

**3 scenarios:**
1. **IT Operations** (no GitHub) — break app → agent detects alert → queries logs → references runbook → remediates
2. **Developer** (requires GitHub) — source code correlation, file:line references, suggested fixes
3. **Workflow** (requires GitHub) — automated issue triage with label classification

**Includes real subagent configs:**
- `incident-handler-core.yaml` — autonomous triage with RunAzCli, QueryLogAnalytics, ExecutePythonCode
- `code-analyzer.yaml` — root cause analysis using runbooks + source code, creates GitHub issues
- `issue-triager.yaml` — classifies and labels GitHub issues following runbook format
- `github-mcp.yaml` — pre-configured GitHub MCP connector

### jiratouchmhp/azure-sre-agent-demo (⭐ 3)

Customer demo with **proactive + reactive** scenarios using Terraform.

**Proactive demos:** security audit (open SQL firewall, public storage), cost review (oversized App Service), availability audit (missing geo-redundancy).

**Reactive demos:** API endpoints to trigger CPU spike, memory leak, slow query, error burst, chaos mode.

**Demo triggers:**
```bash
# Interactive menu
./scripts/trigger-scenarios.sh

# Direct API calls
curl -X POST "https://<api-url>/api/heavy-compute?durationSeconds=30&intensity=85"
curl -X POST "https://<api-url>/api/memory-leak?allocationMb=50&iterations=5"
curl -X POST "https://<api-url>/api/error-burst?count=50"
curl -X POST "https://<api-url>/api/chaos"
```

## Deployment Patterns Summary

| Pattern | Tool | Time | What You Get |
|---------|------|------|-------------|
| Portal wizard | Browser | 5 min | Agent only |
| `azd up` | Azure Developer CLI | 8-12 min | Agent + app + monitoring + subagents + KB |
| Bicep modules | `az deployment` | 15-25 min | Agent + AKS + full observability stack |
| Terraform | `terraform apply` | 10 min | Agent + App Service + PostgreSQL + monitoring |

## Available Regions

SRE Agent is ONLY available in:
- **East US 2** (`eastus2`)
- **Sweden Central** (`swedencentral`)
- **Australia East** (`australiaeast`)

## Cost Estimates

| Configuration | Daily | Monthly |
|--------------|-------|---------|
| SRE Agent only | ~$10 | ~$300 |
| + AKS sandbox (matthansen0) | ~$32-38 | ~$950-1,150 |
| + App Service demo (jiratouchmhp) | ~$15 | ~$460 |

> **Important:** Tear down demo environments after use. Use `azd down --purge`, `terraform destroy`, or the repo's cleanup script.

## Plugin Development

The [Azure/sre-agent-plugins](https://github.com/Azure/sre-agent-plugins) repo provides plugin development templates. Plugins are installable from the [Plugin Marketplace](https://sre.azure.com/docs/capabilities/plugin-marketplace) in the portal.

Browse community plugins at **Builder → Plugin Marketplace** or [sre.azure.com/docs/capabilities/plugin-marketplace](https://sre.azure.com/docs/capabilities/plugin-marketplace).

## For More Details

- `microsoft_docs_fetch(url="https://sre.azure.com/docs/capabilities/plugin-marketplace")`
- `microsoft_docs_search(query="Azure SRE Agent pricing cost estimation")`
- `microsoft_docs_search(query="Azure SRE Agent supported regions availability")`
