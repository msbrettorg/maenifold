# Product Requirements Document: DevAssist AI

## Document Control

| Version | Date | Author | Description |
|---------|------|--------|-------------|
| 1.0 | 2026-01-16 | PM Agent | Initial PRD |
| 1.1 | 2026-01-16 | PM Agent | Added Phase 1/2 structure, CopilotKit patterns, Maenifold integration |
| 1.2 | 2026-01-16 | PM Agent | Added FR-8.x Declarative YAML Workflows from Maenifold assets |

---

## 1. Executive Summary

**Product Name**: DevAssist AI
**Product Type**: Multi-Agent Development & Research Assistant with Persistent Context
**Target Release**: Q1 2026

DevAssist AI is a dual-purpose AI assistant that combines research/analysis and code development capabilities through a coordinated multi-agent workflow. It provides human-in-the-loop approval for tool execution and **persistent context engineering** via Maenifold integration.

### Vision Statement

Enable developers and researchers to accelerate their work through an intelligent assistant that:
- Orchestrates specialized AI agents with real-time streaming
- Maintains human oversight of consequential actions
- **Builds compounding knowledge** through graph-based context that persists across sessions

### Release Phases

| Phase | Focus | Key Deliverables |
|-------|-------|------------------|
| **Phase 1** | Core Agent Workflows + CopilotKit UI | State machine workflows, AG-UI integration, HITL, research canvas |
| **Phase 2** | Declarative YAML Workflows + Maenifold | 32 converted Maenifold reasoning workflows, role/color augmentation, MCP tools, graph-RAG, sequential thinking |

---

## 1.1 Reference Examples (CopilotKit)

Phase 1 implementation draws from these CopilotKit patterns:

| Example | Pattern | Application |
|---------|---------|-------------|
| **[state-machine](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/state-machine)** | Multi-stage workflow with state visualization | Orchestrator routing through agent stages |
| **[research-canvas](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/research-canvas)** | LangGraph-based research agent with canvas UI | Researcher agent with web search |
| **[chat-with-your-data](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/chat-with-your-data)** | useCopilotReadable + backend actions | Shared state, secure backend tools |

---

## 1.2 Context Engineering Principles

Based on [Anthropic's Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents):

> **Context engineering** is finding the *smallest possible set of high-signal tokens* that maximize the likelihood of the desired outcome.

### Key Principles

1. **Context as Finite Resource**: LLMs have limited "attention budgets" - every token depletes it
2. **Right Altitude Prompts**: Not too brittle (hardcoded logic), not too vague (assumes shared context)
3. **Just-in-Time Retrieval**: Maintain lightweight identifiers, load data dynamically at runtime
4. **Compaction**: Summarize context when nearing limits, preserve critical details
5. **Structured Note-Taking**: Persist notes outside context window for later retrieval
6. **Sub-Agent Architectures**: Specialized agents with clean context windows return condensed summaries

---

## 2. Problem Statement

### Current Challenges

1. **Context Switching**: Developers frequently switch between research (docs, Stack Overflow) and coding tasks, breaking flow state
2. **AI Hallucinations**: Single-agent systems lack verification, leading to incorrect code or outdated information
3. **Lack of Oversight**: Fully autonomous agents execute actions without user approval, causing unintended consequences
4. **Poor Integration**: Existing AI tools are standalone, not embedded in developer workflows
5. **No Streaming Feedback**: Batch responses create long waits with no visibility into progress

### Solution

A multi-agent workflow system that:
- Routes tasks to specialized agents (Research, Code, Review)
- Streams real-time progress via AG-UI protocol
- Requires human approval for tool execution (file writes, API calls)
- Embeds directly in web applications via CopilotKit

---

## 3. User Personas

### 3.1 Developer (Primary)

**Name**: Alex
**Role**: Senior Software Engineer
**Goals**:
- Research APIs and libraries quickly without leaving IDE context
- Generate boilerplate code with proper patterns
- Get code reviews before committing

**Pain Points**:
- Spends 30%+ time searching documentation
- AI-generated code often has subtle bugs
- No trusted review process for generated code

### 3.2 Technical Lead (Secondary)

**Name**: Jordan
**Role**: Tech Lead / Architect
**Goals**:
- Evaluate technical approaches with research synthesis
- Prototype solutions rapidly
- Ensure code quality across team

**Pain Points**:
- Manual research for technology decisions
- Reviewing AI-assisted code from team members
- Ensuring consistency in generated code

### 3.3 Researcher (Secondary)

**Name**: Sam
**Role**: Solutions Architect / Analyst
**Goals**:
- Synthesize information from multiple sources
- Create technical documentation
- Analyze competitive solutions

**Pain Points**:
- Information scattered across sources
- Time-consuming manual synthesis
- Difficulty verifying accuracy

---

## 4. Functional Requirements

### 4.1 Agent System

| ID | Requirement | Priority | Agent |
|----|-------------|----------|-------|
| FR-1.1 | System SHALL provide an Orchestrator Agent that routes user requests to appropriate specialist agents | P0 | Orchestrator |
| FR-1.2 | Orchestrator SHALL analyze user intent and classify requests as: research, code, review, or hybrid | P0 | Orchestrator |
| FR-1.3 | System SHALL provide a Researcher Agent that searches, gathers, and synthesizes information | P0 | Researcher |
| FR-1.4 | Researcher Agent SHALL cite sources for all factual claims | P1 | Researcher |
| FR-1.5 | Researcher Agent SHALL support web search tool for real-time information | P0 | Researcher |
| FR-1.6 | System SHALL provide a Coder Agent that generates, modifies, and explains code | P0 | Coder |
| FR-1.7 | Coder Agent SHALL support multiple programming languages (C#, Python, TypeScript, JavaScript) | P0 | Coder |
| FR-1.8 | Coder Agent SHALL generate code following best practices and patterns | P1 | Coder |
| FR-1.9 | System SHALL provide a Reviewer Agent that validates outputs from other agents | P0 | Reviewer |
| FR-1.10 | Reviewer Agent SHALL check code for correctness, security issues, and best practices | P0 | Reviewer |
| FR-1.11 | Reviewer Agent SHALL verify research accuracy and source quality | P1 | Reviewer |

### 4.2 Workflow System

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-2.1 | System SHALL support sequential workflows where agent output flows to the next agent | P0 |
| FR-2.2 | System SHALL support conditional routing based on content analysis | P0 |
| FR-2.3 | System SHALL support fan-out workflows where multiple agents process in parallel | P1 |
| FR-2.4 | Workflows SHALL be composable (workflows can contain other workflows) | P2 |
| FR-2.5 | System SHALL route research requests through: Orchestrator → Researcher → Reviewer | P0 |
| FR-2.6 | System SHALL route code requests through: Orchestrator → Coder → Reviewer | P0 |
| FR-2.7 | System SHALL route hybrid requests through: Orchestrator → Researcher → Coder → Reviewer | P1 |

### 4.3 Human-in-the-Loop (HITL)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-3.1 | System SHALL require user approval before executing file write operations | P0 |
| FR-3.2 | System SHALL require user approval before executing external API calls | P0 |
| FR-3.3 | System SHALL require user approval before executing code/shell commands | P0 |
| FR-3.4 | Approval requests SHALL display: action type, target, parameters, and risk level | P0 |
| FR-3.5 | User SHALL be able to approve, reject, or modify tool parameters | P0 |
| FR-3.6 | System SHALL timeout approval requests after configurable duration (default: 5 minutes) | P1 |
| FR-3.7 | System SHALL support "approve all similar" for batch operations | P2 |

### 4.4 AG-UI Integration

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-4.1 | System SHALL expose agent workflows via AG-UI protocol over HTTP/SSE | P0 |
| FR-4.2 | System SHALL stream agent responses in real-time as tokens are generated | P0 |
| FR-4.3 | System SHALL maintain conversation thread context across requests | P0 |
| FR-4.4 | System SHALL surface tool approval requests as FunctionCallContent events | P0 |
| FR-4.5 | System SHALL support shared state synchronization between client and server | P1 |
| FR-4.6 | System SHALL support predictive state updates for optimistic UI | P2 |

### 4.5 User Interface

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-5.1 | UI SHALL provide a chat interface for user-agent interaction | P0 |
| FR-5.2 | UI SHALL display streaming responses with typing indicators | P0 |
| FR-5.3 | UI SHALL display which agent is currently responding | P0 |
| FR-5.4 | UI SHALL provide approval dialogs for HITL tool calls | P0 |
| FR-5.5 | UI SHALL render code blocks with syntax highlighting | P0 |
| FR-5.6 | UI SHALL support markdown formatting in responses | P0 |
| FR-5.7 | UI SHALL display workflow progress indicator | P1 |
| FR-5.8 | UI SHALL provide state panel showing current context | P2 |

### 4.6 Tools

| ID | Requirement | Priority | Tool |
|----|-------------|----------|------|
| FR-6.1 | System SHALL provide web search tool for Researcher Agent | P0 | WebSearch |
| FR-6.2 | System SHALL provide file read tool for all agents | P0 | FileRead |
| FR-6.3 | System SHALL provide file write tool with HITL approval | P0 | FileWrite |
| FR-6.4 | System SHALL provide code execution tool with HITL approval | P1 | CodeExec |
| FR-6.5 | System SHALL provide API call tool with HITL approval | P1 | ApiCall |
| FR-6.6 | Tools SHALL return structured output for agent consumption | P0 | All |

### 4.7 Context Engineering (Phase 2)

| ID | Requirement | Priority | Maenifold Tool |
|----|-------------|----------|----------------|
| FR-7.1 | System SHALL maintain a knowledge graph built from `[[WikiLink]]` concepts in agent responses | P0 | `ma:writememory`, `ma:sync` |
| FR-7.2 | System SHALL support hybrid search (semantic + full-text) across persisted knowledge | P0 | `ma:searchmemories` |
| FR-7.3 | System SHALL traverse concept relationships for context building | P0 | `ma:buildcontext` |
| FR-7.4 | System SHALL discover semantically similar concepts via embeddings | P1 | `ma:findsimilarconcepts` |
| FR-7.5 | System SHALL persist structured notes to `memory://` with source lineage | P0 | `ma:writememory` |
| FR-7.6 | System SHALL support sequential thinking sessions with revision/branching | P1 | `ma:sequentialthinking` |
| FR-7.7 | System SHALL compact context when approaching token limits, preserving critical details | P0 | Compaction middleware |
| FR-7.8 | System SHALL use just-in-time retrieval vs pre-loading entire context | P0 | Agent middleware |
| FR-7.9 | System SHALL share thinking sessions between orchestrator and sub-agents | P1 | `ma:sequentialthinking` |
| FR-7.10 | System SHALL support workflow orchestration for systematic reasoning | P2 | `ma:workflow` |
| FR-7.11 | System SHALL extract `[[concepts]]` from agent responses for graph construction | P0 | `ma:extractconceptsfromfile` |
| FR-7.12 | System SHALL cite `memory://` URIs for knowledge-grounded responses | P0 | Response formatting |

### 4.8 Declarative YAML Workflows (Phase 2)

Based on [Microsoft Agent Framework Declarative Workflows](https://github.com/microsoft/agent-framework/tree/main/workflow-samples):

| ID | Requirement | Priority | Source |
|----|-------------|----------|--------|
| FR-8.1 | System SHALL load declarative YAML workflow definitions at startup | P0 | `Services/WorkflowLoader.cs` |
| FR-8.2 | System SHALL support Agent, LLMCall, Condition, and Loop node types in YAML | P0 | Agent Framework YAML spec |
| FR-8.3 | System SHALL match user intent to workflows via trigger keywords | P0 | `Services/WorkflowRouter.cs` |
| FR-8.4 | System SHALL convert 32 Maenifold JSON workflows to YAML format | P0 | `/maenifold/src/assets/workflows/` |
| FR-8.5 | YAML workflows SHALL preserve Maenifold metadata (toolHints, stop_conditions, guardrails) | P0 | Schema mapping |
| FR-8.6 | System SHALL augment agent instructions with Role personality data | P1 | `Services/RoleAugmenter.cs` |
| FR-8.7 | System SHALL augment agent instructions with Color archetype traits | P1 | Colors JSON → YAML |
| FR-8.8 | YAML workflow nodes SHALL declare MCP tools for Maenifold integration | P0 | Tool declarations |
| FR-8.9 | System SHALL support Power Fx expressions for conditional routing in YAML | P1 | Agent Framework spec |
| FR-8.10 | System SHALL support workflow hot-reload without recompilation | P2 | File watcher |

### 4.8.1 Maenifold Asset Conversion

| Asset Type | Count | Target Format | Key Fields |
|------------|-------|---------------|------------|
| `workflows/` | 32 | `Workflows/Declarative/*.yaml` | nodes, edges, triggers, metadata |
| `roles/` | 16 | `Workflows/Roles/*.yaml` | personality.principles, approach, antiPatterns |
| `colors/` | 7 | `Workflows/Colors/*.yaml` | archetype, traits, neuralActivation |

### 4.8.2 YAML Workflow Schema Mapping

| Maenifold JSON | Agent Framework YAML | Notes |
|----------------|---------------------|-------|
| `id` | `nodes[].id` | Becomes node identifier |
| `name` | `name` (workflow) | Top-level workflow name |
| `triggers[]` | `triggers[]` | Used for intent routing |
| `steps[]` | `nodes[]` | Each step → Agent node |
| `steps[].description` | `nodes[].instructions` | System prompt for step |
| `steps[].requiresEnhancedThinking` | `nodes[].tools[]` | Add SequentialThinking tool |
| `metadata.toolHints` | `nodes[].tools[]` | Extract tool names |
| `metadata.stop_conditions` | `metadata.stop_conditions` | Preserve in metadata |
| `metadata.guardrails` | `metadata.guardrails` | Preserve in metadata |

### 4.9 Context Engineering Patterns

Based on Maenifold's RAG Technique Support:

| Pattern | Description | Implementation |
|---------|-------------|----------------|
| **HYDE** | Hypothetical Document Embeddings - generate synthetic answer, search for supporting evidence | Query expansion via semantic search |
| **Graph-RAG** | Traverse concept relationships before retrieval | `ma:buildcontext` with depth parameter |
| **Self-RAG** | Evaluate retrieved context relevance before use | Reviewer agent validation |
| **FLARE** | Forward-Looking Active REtrieval - fetch just-in-time during generation | Tool call patterns |
| **Contextual Compression** | Summarize when nearing limits | Compaction middleware |

---

## 5. Non-Functional Requirements

### 5.1 Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-1.1 | Time to first token (TTFT) | < 500ms |
| NFR-1.2 | Agent routing decision | < 100ms |
| NFR-1.3 | Workflow state transitions | < 50ms |
| NFR-1.4 | UI responsiveness during streaming | 60 FPS |
| NFR-1.5 | Concurrent users per instance | >= 100 |

### 5.2 Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-2.1 | System uptime | 99.9% |
| NFR-2.2 | Graceful degradation on Azure Foundry unavailability | Required |
| NFR-2.3 | Automatic retry on transient failures | 3 attempts |
| NFR-2.4 | Session recovery after disconnect | Within 30s |

### 5.3 Security

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-3.1 | Authentication | Azure AD / Entra ID |
| NFR-3.2 | Authorization | Role-based (user, admin) |
| NFR-3.3 | Data encryption in transit | TLS 1.3 |
| NFR-3.4 | Prompt injection protection | Input sanitization |
| NFR-3.5 | Tool execution sandboxing | Container isolation |
| NFR-3.6 | Audit logging | All tool executions |

### 5.4 Scalability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-4.1 | Horizontal scaling | Kubernetes-ready |
| NFR-4.2 | Stateless backend | Required |
| NFR-4.3 | Session affinity | Sticky sessions supported |

---

## 6. Agent Specifications

### 6.1 Orchestrator Agent

**Purpose**: Route user requests to appropriate specialist agents based on intent analysis.

**System Prompt**:
```
You are the Orchestrator Agent. Your role is to analyze user requests and route them to the appropriate specialist agent(s).

Classify each request as one of:
- RESEARCH: Questions about concepts, documentation, best practices, comparisons
- CODE: Requests to write, modify, explain, or debug code
- REVIEW: Requests to review existing code or content
- HYBRID: Complex requests requiring both research and code generation

For RESEARCH requests, route to Researcher Agent.
For CODE requests, route to Coder Agent.
For REVIEW requests, route to Reviewer Agent.
For HYBRID requests, route to Researcher Agent first, then Coder Agent.

Always explain your routing decision briefly.
```

**Tools**: None (routing only)

### 6.2 Researcher Agent

**Purpose**: Search, gather, and synthesize information from multiple sources.

**System Prompt**:
```
You are the Researcher Agent. Your role is to find accurate, up-to-date information and synthesize it for the user.

Guidelines:
1. Always cite your sources with URLs when available
2. Distinguish between facts and opinions
3. Acknowledge uncertainty when information is conflicting
4. Prioritize official documentation over community sources
5. Summarize key findings in bullet points

When using the web search tool, craft specific queries for better results.
```

**Tools**: WebSearch, FileRead

### 6.3 Coder Agent

**Purpose**: Generate, modify, and explain code across multiple languages.

**System Prompt**:
```
You are the Coder Agent. Your role is to write high-quality code that solves user problems.

Guidelines:
1. Follow language-specific best practices and conventions
2. Include meaningful comments for complex logic
3. Handle errors appropriately
4. Write testable, modular code
5. Explain your implementation approach

When writing files, use the FileWrite tool (requires user approval).
When executing code, use the CodeExec tool (requires user approval).
```

**Tools**: FileRead, FileWrite (HITL), CodeExec (HITL)

### 6.4 Reviewer Agent

**Purpose**: Validate and improve outputs from other agents.

**System Prompt**:
```
You are the Reviewer Agent. Your role is to ensure quality, accuracy, and security of all outputs.

For Code Reviews:
1. Check for correctness and logic errors
2. Identify security vulnerabilities (OWASP Top 10)
3. Verify error handling
4. Assess code style and maintainability
5. Suggest improvements

For Research Reviews:
1. Verify source credibility
2. Check for outdated information
3. Identify missing perspectives
4. Validate factual claims

Provide actionable feedback with specific suggestions.
```

**Tools**: FileRead

---

## 7. Technical Architecture

### 7.1 System Context (Phase 1)

```
┌─────────────────────────────────────────────────────────────────┐
│                         DevAssist AI                            │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────────┐    ┌───────────────┐  │
│  │   React +   │◄──►│  ASP.NET Core    │◄──►│ Azure AI      │  │
│  │  CopilotKit │SSE │  AG-UI Server    │    │ Foundry       │  │
│  └─────────────┘    └──────────────────┘    └───────────────┘  │
│         │                   │                       │           │
│         │                   ▼                       │           │
│         │          ┌──────────────────┐             │           │
│         │          │  Agent Workflow  │             │           │
│         │          │    Orchestrator  │             │           │
│         │          └────────┬─────────┘             │           │
│         │                   │                       │           │
│         │     ┌─────────────┼─────────────┐         │           │
│         │     ▼             ▼             ▼         │           │
│         │ ┌───────┐   ┌───────┐   ┌──────────┐     │           │
│         │ │Researcher│ │ Coder │   │ Reviewer │     │           │
│         │ └───────┘   └───────┘   └──────────┘     │           │
└─────────────────────────────────────────────────────────────────┘
```

### 7.2 System Context (Phase 2 - Maenifold Integration)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         DevAssist AI + Maenifold                            │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌──────────────────┐    ┌───────────────┐               │
│  │   React +   │◄──►│  ASP.NET Core    │◄──►│ Azure AI      │               │
│  │  CopilotKit │SSE │  AG-UI Server    │    │ Foundry       │               │
│  └─────────────┘    └──────────────────┘    └───────────────┘               │
│                              │                                              │
│                              ▼                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐  │
│  │                    Maenifold Context Layer                            │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐               │  │
│  │  │ Graph DB    │  │ Sequential   │  │ Hybrid Search  │               │  │
│  │  │[[WikiLinks]]│  │ Thinking     │  │ (Semantic+FTS) │               │  │
│  │  └─────────────┘  └──────────────┘  └────────────────┘               │  │
│  │  ┌─────────────┐  ┌──────────────┐  ┌────────────────┐               │  │
│  │  │ memory://   │  │ Workflows    │  │ Compaction     │               │  │
│  │  │ Persistence │  │ (30 methods) │  │ Middleware     │               │  │
│  │  └─────────────┘  └──────────────┘  └────────────────┘               │  │
│  └──────────────────────────────────────────────────────────────────────┘  │
│                              │                                              │
│                              ▼                                              │
│          ┌──────────────────────────────────────┐                          │
│          │          Agent Workflow              │                          │
│          │   (Orchestrator + Specialists)       │                          │
│          │                                      │                          │
│          │   Agents share:                      │                          │
│          │   • [[concept]] extraction           │                          │
│          │   • Sequential thinking sessions     │                          │
│          │   • memory:// citations              │                          │
│          └──────────────────────────────────────┘                          │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 7.3 Data Flow (Phase 1)

1. User submits request via CopilotKit chat UI
2. Request sent to AG-UI endpoint via HTTP POST
3. Orchestrator Agent analyzes intent and routes to specialist(s)
4. Specialist agent(s) process request, invoking tools as needed
5. Tool calls requiring approval are surfaced as FunctionCallContent
6. User approves/rejects via ApprovalDialog
7. Upon approval, tool executes and returns result
8. Response streams back via SSE to CopilotKit
9. Final output displayed in chat UI

### 7.4 Data Flow (Phase 2 - Context Engineering)

1. User submits request via CopilotKit chat UI
2. **Context Layer**: `ma:searchmemories` retrieves relevant `memory://` notes
3. **Context Layer**: `ma:buildcontext` traverses `[[concepts]]` for related knowledge
4. Request + enriched context sent to AG-UI endpoint
5. Orchestrator routes to specialists with shared `ma:sequentialthinking` session
6. Specialist agents:
   - Extract `[[concepts]]` from responses for graph construction
   - Cite `memory://` URIs for grounded responses
   - Call `ma:writememory` to persist new knowledge
7. **Compaction**: If context nearing limits, summarize and persist to `memory://`
8. Response streams with `[[concept]]` markup for graph building
9. Graph syncs via `ma:sync` for future retrieval

### 7.5 Technology Stack

| Layer | Technology |
|-------|------------|
| Frontend | React 18, TypeScript, Vite, CopilotKit |
| Protocol | AG-UI (SSE over HTTP) |
| Backend | .NET 8, ASP.NET Core |
| Agent Framework | Microsoft.Agents.AI.Workflows |
| AI Provider | Azure AI Foundry Agents |
| **Context Engineering** | **Maenifold MCP (Graph DB, Embeddings, Workflows)** |
| Authentication | Azure AD / Entra ID |
| Hosting | Azure App Service / AKS |

---

## 8. Success Metrics

### 8.1 Adoption Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Daily Active Users | 100+ | Analytics |
| Sessions per User | 3+ | Analytics |
| Retention (Day 7) | 50% | Cohort analysis |

### 8.2 Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Tool Approval Rate | > 80% | Logs |
| Research Accuracy | > 90% | User feedback |
| Code Compilation Success | > 95% | Tool results |
| User Satisfaction (CSAT) | > 4.0/5 | Survey |

### 8.3 Performance Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to First Token | < 500ms | APM |
| End-to-End Latency | < 10s (p50) | APM |
| Error Rate | < 1% | Logs |
| Uptime | 99.9% | Monitoring |

---

## 9. Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| Azure Foundry rate limits | High | Medium | Implement queuing, caching |
| Prompt injection attacks | High | Medium | Input validation, sandboxing |
| Agent hallucinations | Medium | High | Reviewer agent, HITL |
| SSE connection drops | Medium | Medium | Automatic reconnection |
| CopilotKit breaking changes | Medium | Low | Pin versions, monitor releases |

---

## 10. Out of Scope (v1)

1. Voice input/output
2. Multi-modal (image) support
3. Custom agent creation by users
4. Offline/local LLM support
5. Mobile native apps
6. Multi-user collaboration

> **Note**: Agent memory/learning across sessions is IN SCOPE for Phase 2 via Maenifold integration.

---

## 11. Dependencies

| Dependency | Owner | Status | Phase |
|------------|-------|--------|-------|
| Azure AI Foundry access | Azure team | Required | 1 |
| Azure OpenAI quota | Azure team | Required | 1 |
| CopilotKit license | Open source | MIT | 1 |
| Microsoft Agent Framework | Microsoft | Preview | 1 |
| **Maenifold MCP Server** | ma-collective | Required | 2 |
| **Maenifold Graph DB** | ma-collective | Required | 2 |
| **Embedding Model** | Azure OpenAI | Required | 2 |

---

## 12. Appendix

### A. Glossary

| Term | Definition |
|------|------------|
| AG-UI | Agent-User Interaction Protocol - standardized protocol for agent-client communication |
| HITL | Human-in-the-Loop - requiring human approval for agent actions |
| SSE | Server-Sent Events - HTTP-based streaming protocol |
| Orchestrator | Agent that routes requests to specialist agents |
| Workflow | Directed graph of agents with edges defining data flow |
| **WikiLink** | `[[concept]]` markup that becomes a node in the knowledge graph |
| **memory://** | URI scheme for Maenifold-persisted knowledge files |
| **Maenifold** | Context engineering infrastructure with graph DB, embeddings, and workflows |
| **Sequential Thinking** | Multi-step reasoning sessions with revision and branching |
| **Compaction** | Summarizing context when approaching token limits |
| **Graph-RAG** | Traversing concept relationships before retrieval |
| **HYDE** | Hypothetical Document Embeddings - synthetic answer generation for search |

### B. References

**Phase 1**:
- [Microsoft Agent Framework Documentation](https://learn.microsoft.com/en-us/agent-framework/)
- [AG-UI Protocol Specification](https://docs.ag-ui.com/)
- [CopilotKit Documentation](https://docs.copilotkit.ai/)
- [Azure AI Foundry](https://learn.microsoft.com/en-us/azure/ai-foundry/)
- [CopilotKit state-machine Example](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/state-machine)
- [CopilotKit research-canvas Example](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/research-canvas)
- [CopilotKit chat-with-your-data Example](https://github.com/CopilotKit/CopilotKit/tree/main/examples/v1.x/chat-with-your-data)

**Phase 2**:
- [Anthropic: Effective Context Engineering for AI Agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
- [Maenifold README](../maenifold/docs/README.md)
- [Maenifold Search and Scripting](../maenifold/docs/search-and-scripting.md)
