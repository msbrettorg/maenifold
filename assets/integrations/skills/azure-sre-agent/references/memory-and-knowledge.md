# Memory & Knowledge Deep Reference

Detailed reference for Azure SRE Agent's memory and knowledge system. This supplements the core SKILL.md with information too detailed for inline inclusion.

## How Memory Search Works

When you ask a question, the agent searches across all knowledge sources simultaneously:

| Source | What it Finds | Best For |
|--------|--------------|----------|
| Past incidents | Steps that resolved similar issues | "How did we fix this before?" |
| User memories | Facts explicitly saved via `#remember` | "Remember my environment uses..." |
| Knowledge base | Uploaded runbooks and docs | "Follow our standard procedure" |

The agent returns a **grounded response with clickable citations** showing exactly where information came from.

## Same-Resource Priority

When investigating a resource issue, the agent prioritizes past sessions on the **exact same resource**. For example, asking about `app-prod-01` returning 503 errors — the agent first checks past sessions on `app-prod-01` specifically.

## Proactive Knowledge Persistence

The agent maintains `memories/synthesizedKnowledge/`:

### overview.md (Always Loaded)

The `overview.md` file is automatically loaded into the system prompt at conversation start. It has a ~2,000 character budget. It serves as:
- Service summary and index
- Links to detailed topic files
- The agent's "at-a-glance" context

### Topic Files

Detailed notes on specific subjects. Updated by merging new information and removing outdated content.

| File | Content | Budget |
|------|---------|--------|
| `overview.md` | Service summary, key links, index | ~2,000 chars |
| `team.md` | Team members, roles, expertise | ~500 chars |
| `architecture.md` | Components, connections, environments | ~1,500 chars |
| `logs.md` | Log sources, tables, key fields, useful queries | ~1,500 chars |
| `deployment.md` | Pipeline details, version lookup, rollback | ~1,000 chars |
| `auth.md` | Auth mechanisms, identity flows | ~800 chars |
| `debugging.md` | Common issues, troubleshooting guides | ~1,000 chars |
| `queries/*.md` | Extracted queries organized by topic | ~1,000 chars each |

### What Gets Saved Proactively

| Category | Examples |
|----------|----------|
| Problem constraints | "This service can't scale past 10 replicas due to quota limits" |
| Strategies that worked | "Restarting the pod with `--grace-period=0` resolved the stuck deployment" |
| Strategies that failed | "Increasing memory limit didn't help — the issue was CPU throttling" |
| Non-obvious dependencies | "app-frontend depends on a sidecar proxy that must start first" |
| Configuration details | "Production uses custom TLS certificates stored in Key Vault" |

### Explicit Save

```
Save this to your knowledge: our Redis cache uses Premium tier with 6GB,
and failover takes about 90 seconds.
```

The agent creates or updates the appropriate knowledge file and links it from `overview.md`.

## Session Insights

Generated automatically after every thread:

| Component | What it Captures | Example |
|-----------|-----------------|---------|
| Symptoms observed | Error patterns, behaviors | "HTTP 503 errors, memory at 95%" |
| Resolution steps | What worked | "Scaled up App Service SKU" |
| Root cause | Why it happened | "Memory leak in deployment v2.3" |
| Pitfalls to avoid | What didn't work | "Restarting didn't help" |

**Timing:**

| Thread Type | When | Trigger |
|-------------|------|---------|
| Sync chat | 30 min after last message | Automatic |
| Async tasks | 30 min after completion | Automatic |
| User feedback | When you rate a response | Manual |

**View insights:** Monitor → Session insights → Timeline, evaluation scores, key learnings, source thread links.

## Knowledge Base Upload

### Supported Formats

Markdown, plain text, PDF, Word, PowerPoint, Excel, images, and more. Max 16 MB per file.

### Best Document Types

| Document Type | Good For |
|---------------|----------|
| Runbooks | Step-by-step incident procedures |
| Architecture guides | Understanding your environment |
| On-call playbooks | Escalation and response procedures |
| API documentation | Service-specific knowledge |
| Team procedures | Workflow and process docs |

### Upload vs Connect

| Upload | Connect via Connector |
|--------|----------------------|
| Incident runbooks | Live wiki pages (ADO, GitHub) |
| Architecture diagrams | Source code repositories |
| Escalation procedures | Real-time monitoring data |
| Static API docs | Frequently updated docs |

## For More Details

- `microsoft_docs_fetch(url="https://learn.microsoft.com/azure/sre-agent/memory")`
- `microsoft_docs_search(query="Azure SRE Agent upload knowledge documents supported formats")`
- `microsoft_docs_search(query="Azure SRE Agent connect knowledge external sources")`
