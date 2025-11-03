# maenifold

**Test-time adaptive reasoning for AI agents.** maenifold enables AI to think through complex problems systematically, building a persistent knowledge graph that compounds over time.

---

**maenifold enables persistent AI reasoning through knowledge graphs.** Where typical AI interactions start fresh each time, maenifold maintains context across sessions through a growing graph of relationships. Every `[[WikiLink]]` creates connections that persist beyond individual conversations.

**The system operates at documented scale**: 1.1M+ graph relationships, 30+ reasoning methodologies, and 85% success rate in our comprehensive E2E testing. Our [Hero Demo](demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md) orchestrated 12 agents across 4 waves and found actual bugs that traditional mocks would miss—including file extension loss during move operations.

**Built on 間 (Ma) principles**: The space between thoughts becomes knowledge. maenifold creates room for AI to think systematically through sequential reasoning sessions that can revise, branch, and persist across days. Not forcing intelligence, but creating space for it to emerge. The name "maenifold" represents the multi-dimensional topology of knowledge—Ma (間) + manifold—where thoughts fold through dimensional space creating persistent reasoning patterns.

**This is test-time reasoning infrastructure.** A foundation for AI agents that build knowledge over time rather than starting from zero. Real markdown files you can read, real SQLite you can query, real tool orchestration that scales to production workloads.

---

間 The space between thoughts becomes knowledge
∴ Knowledge compounds into wisdom

## 📍 Table of Contents

### Core Understanding
- [1. Beyond RAG: Real Reasoning](#1-beyond-rag-real-reasoning)
- [2. How It Works](#2-how-it-works)
  - [2.1 Sequential Thinking](#21-sequential-thinking)
  - [2.2 Orchestrated Workflows](#22-orchestrated-workflows)
  - [2.3 Hybrid RRF Search](#23-hybrid-rrf-search)
  - [2.4 Lazy Graph Construction](#24-lazy-graph-construction)

### Getting Started
- [3. Quick Start](#3-quick-start)
  - [3.1 For VSCode Users](#31-for-vscode-users)
  - [3.2 For Developers](#32-for-developers)

### Architecture & Capabilities
- [4. The Cognitive Stack](#4-the-cognitive-stack)
  - [4.1 Memory Layer](#41-memory-layer)
  - [4.2 Graph Layer](#42-graph-layer)
  - [4.3 Reasoning Layer](#43-reasoning-layer)
- [5. Key Capabilities](#5-key-capabilities)
- [6. Technical Specifications](#6-technical-specifications)

### Real-World Usage
- [7. Example Usage](#7-example-usage)
  - [7.1 The PM Protocol](#71-the-pm-protocol)
  - [7.2 Complex Problem Solving](#72-complex-problem-solving)
  - [7.3 Multi-Agent Development](#73-multi-agent-development)
  - [7.4 Knowledge Graph Queries](#74-knowledge-graph-queries)
  - [7.5 Observing AI Cognition](#75-observing-ai-cognition)
- [8. Real-World Impact](#8-real-world-impact)
  - [8.1 What This Enables](#81-what-this-enables)
  - [8.2 The Compound Effect](#82-the-compound-effect)

### Testing & Validation
- [9. Real Testing, Real Bugs, Real Confidence](#9-real-testing-real-bugs-real-confidence)
  - [9.1 The Hero Demo](#91-the-hero-demo)
  - [9.2 Why This Matters](#92-why-this-matters)
  - [9.3 Adaptive Test Evolution](#93-adaptive-test-evolution)
  - [9.4 Test Philosophy](#94-test-philosophy)

### Philosophy & Documentation
- [10. Technical Principles](#10-technical-principles)
- [11. Project Structure](#11-project-structure)
- [12. Documentation](#12-documentation)
- [13. License & Attribution](#13-license--attribution)

## 1. Beyond RAG: Real Reasoning

maenifold isn't just retrieval—it's a cognitive architecture for AI agents that need to:

- **Think through multi-step problems** with sequential thinking sessions that can revise and branch
- **Select optimal reasoning approaches** through intelligent workflow dispatch and meta-cognitive analysis
- **Orchestrate complex workflows** with 30 distinct methodologies from deductive reasoning to design thinking
- **Build knowledge that compounds** through a lazily-constructed graph that emerges from use
- **Coordinate multi-agent systems** with quality gates, validation waves, and sophisticated process control
- **Maintain complete transparency** with Obsidian-compatible markdown files for every thought

This implements **test-time compute for systematic problem-solving** - dynamically allocating reasoning resources based on problem complexity and cognitive requirements.

## 2. How It Works

### 2.1 Sequential Thinking

AI agents can engage in multi-hour reasoning sessions with full revision capability:

```markdown
# Thinking Session: Architecture Design
Thought 1: Analyzing requirements...
Thought 2: Wait, I need to reconsider the scaling approach from Thought 1...
Thought 3: Branching to explore microservices vs monolithic...
```

Each session creates a `memory://thinking/session-{id}` file tracking the complete reasoning chain. Sessions support:
- **Revision**: Reconsider and update previous thoughts
- **Branching**: Explore alternative reasoning paths
- **Persistence**: Continue across days or weeks
- **Multi-agent**: Share sessions between agents

### 2.2 Orchestrated Workflows

Pre-built workflows embed sequential thinking at critical decision points:

- **Discovery Wave**: Parallel agents explore the problem space
- **Validation Wave**: Test assumptions and verify approaches
- **Implementation Wave**: Execute with confidence

**Intelligent Workflow Selection**: The `workflow-dispatch` meta-system analyzes problem characteristics, researches historical context, assesses cognitive requirements, and automatically selects optimal reasoning methodologies. This meta-cognitive system implements **test-time compute for systematic problem-solving**.

The **PM Pattern**: A primary agent (usually Sonnet) acts as the blue-hat orchestrator, using sequential thinking to maintain project context. It dispatches ephemeral sub-agents for specific tasks—these agents can burn through their context windows on implementation details while the PM preserves the overall vision. All thinking persists to `memory://` for continuity.

Workflows maintain state across days, enabling true long-running projects.

### 2.3 Hybrid RRF Search

Combines semantic vectors with full-text search using Reciprocal Rank Fusion:

- **Semantic search** finds conceptually related content
- **Full-text search** ensures exact terms aren't missed
- **RRF fusion** optimally blends both result sets (k=60)
- **Returns context** with relevance scores for transparency

This dual approach ensures you never miss exact matches while still finding conceptually similar content.

### 2.4 Lazy Graph Construction

The knowledge graph builds itself through natural use:

```markdown
# API Design Decision
We chose [[REST]] over [[GraphQL]] for our [[public API]] due to
[[caching]] requirements and [[client simplicity]].
```

Every `[[WikiLink]]` becomes a node. Every mention strengthens edges. Patterns emerge without planning.

∴ Structure emerges from use, not from schema

## 3. Quick Start

### 3.1 For VSCode Users

1. **Install maenifold**:
```bash
npm install -g @ma-collective/maenifold
```

2. **Configure your AI assistant** to use maenifold:

**Single-agent setup** (Continue, Cline, etc.):

#### Continue.dev Configuration
Add to `~/.continue/config.json`:
```json
{
  "models": [...],
  "mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": {
        "MAENIFOLD_ROOT": "~/maenifold"
      }
    }
  }
}
```

#### Cline (Claude Dev) Configuration
Add to VSCode settings (`Cmd+,` → Extensions → Cline):
```json
{
  "cline.mcpServers": {
    "maenifold": {
      "command": "maenifold",
      "args": ["--mcp"],
      "env": {
        "MAENIFOLD_ROOT": "~/maenifold"
      }
    }
  }
}
```

#### Codex Configuration
Add to `~/.codex/config.toml`:
```toml
[mcp_servers.maenifold]
type = "stdio"
command = "maenifold"
args = ["--mcp"]
startup_timeout_sec = 120
tool_timeout_sec = 600
env = { MAENIFOLD_ROOT = "~/maenifold" }
```

**Multi-agent orchestration** requires MCP clients with agent dispatch:
- **Claude Code** - `claude-code` CLI with Task tool
- **aishell** - Open-source with agent management
- **Copilot CLI** - GitHub's multi-agent interface
- **Codex** - Purpose-built orchestrator

3. **Start using it in VSCode**:
- Single agent: "Write a memory about our project architecture"
- Multi-agent: "Use agentic-dev workflow to implement authentication"
- Graph query: "Show me how our concepts connect"

### 3.2 For Developers

```bash
# CLI mode - direct tool access
maenifold --tool writememory --payload '{"title":"Meeting Notes","content":"# Meeting Notes\n\nDiscussed [[architecture]] and [[performance]]"}'

# MCP server mode
maenifold --mcp
```

## 4. The Cognitive Stack

### 4.1 Memory Layer (`memory://`)

Every piece of knowledge lives as a markdown file with a unique URI:
- `memory://decisions/api-design` - Architectural decisions
- `memory://thinking/session-12345` - Sequential thinking sessions
- `memory://research/rag-comparison` - Research notes

All files are human-readable, Obsidian-compatible, and persist across sessions.

### 4.2 Graph Layer (SQLite + Vectors)

Automatic graph construction from WikiLinks with:
- **384-dimensional embeddings** for semantic similarity
- **Edge weights** that strengthen with repeated mentions
- **Concept clustering** revealing emergent patterns
- **Incremental sync** keeping the graph current

### 4.3 Reasoning Layer (Tools + Workflows)

Where test-time computation happens:
- **Sequential Thinking**: Multi-step reasoning with revision and branching
- **Workflow Orchestration**: 30 distinct methodologies with quality gates and guardrails
- **Assumption Ledger**: Traceable skepticism for agent reasoning—capture, validate, and track assumptions without auto-inference
- **Multi-agent Coordination**: Wave-based execution with parallel agent dispatch
- **Intelligent Method Selection**: Meta-cognitive system for optimal reasoning approach selection
- **RTM Validation**: Requirements traceability for systematic development
- **Quality Control**: Stop conditions, validation gates, and anti-slop controls

**Context Window Economics**: The PM (blue hat) uses sequential thinking to preserve expensive context while dispatching fresh agents for implementation. This allows complex projects without context exhaustion.

∴ The PM remembers so agents can forget

## 5. Key Capabilities

- **Test-time Adaptive Reasoning**: Sequential thinking with revision, branching, and multi-agent collaboration
- **Intelligent Workflow Selection**: Meta-cognitive system that analyzes problems and selects optimal reasoning approaches
- **30 Distinct Methodologies**: Complete taxonomy from deductive reasoning to design thinking, with sophisticated orchestration
- **Hybrid RRF Search**: Semantic + full-text fusion for optimal retrieval (not just embedding similarity)
- **Lazy Graph Construction**: No schema, no ontology—structure emerges from WikiLink usage
- **Quality-Gated Orchestration**: Multi-agent coordination with validation waves, guardrails, and RTM compliance
- **Complete Transparency**: Every thought, revision, and decision visible in markdown files
- **Multi-day Persistence**: Sessions maintain state across restarts, enabling long-running projects

## 6. Technical Specifications

- **Language**: C# with .NET 9.0
- **Vector Dimensions**: 384 (all-MiniLM-L6-v2 via ONNX)
- **Search Algorithm**: Reciprocal Rank Fusion (k=60)
- **Database**: SQLite with vector extension
- **Graph Sync**: Incremental with file watching
- **Memory Format**: Markdown with YAML frontmatter
- **URI Scheme**: `memory://` protocol
- **Tested Scale**: > 1.1 million relationships
- **Cognitive Assets**:
  - **30 Distinct Methodologies**: Complete taxonomy from deductive reasoning to design thinking
    - Reasoning: deductive, inductive, abductive, critical, strategic, higher-order thinking
    - Creative: design thinking, divergent thinking, lateral thinking, oblique strategies, SCAMPER
    - Development: agentic-dev with anti-slop controls, agile, SDLC, code review workflows
    - Collaborative: world café, parallel thinking, six thinking hats
    - Meta-orchestration: workflow-dispatch for intelligent methodology selection
  - **7 Roles** (architect, engineer, PM, etc.)
  - **7 Thinking Hats** (DeBono's Six + Gray)
- **MCP Compliance**: Full tool annotation support

## 7. Example Usage

### 7.1 The PM Protocol (Orchestration Pattern)

Here's a battle-tested prompt for multi-agent orchestration:

```markdown
## Your PM Protocol

Perform all tasks below with extreme precision and care. Follow the protocol exactly.

### Initial Setup
1) Adopt blue hat
2) Adopt product manager role
3. Read the product documentation/specifications carefully
4) Restate the user's task to refine scope and remove ambiguity

### Task Execution
- Use waves of concurrent 'agents' to complete the user's task - you are the overseer of the entire process.
- Do NOT dispatch agents sequentially - they WILL block you till they complete. Use concurrent agents instead.
- Your agents are ephemeral. Do not expect them to remember anything from previous tasks.
- Use shared workflows and sequential thinking sessions to communicate with your agents and ground them in their task.
- Pay attention to session ID's for workflow and sequential thinking sessions and provide them to your agents in their instructions.
- Require workflow/sequential thinking evidence from agents documenting their discoveries using Codenav and Context7.
- Never accept an agent's report of success. ALWAYS verify agent work by dispatching follow-up agents using CodeNav + Context7 + Sequential Thinking
- The build must compile and pass all tests before reporting success.
- RTM is MANDATORY for ALL code changes, and all changes must link to a specific requirement
- We build products that are Simple, Lovable and Complete. Do not accept partial solutions or MVPs.
```

### 7.2 Complex Problem Solving

```
You: "Design a distributed cache for our microservices"
AI: [Initiates sequential thinking session]
    → Thought 1: Analyzing consistency requirements
    → Thought 2: Evaluating Redis vs Hazelcast
    → Thought 3: Revising Thought 1 based on latency data
    → Creates memory://decisions/cache-architecture
    → Links to [[distributed-systems]], [[caching]], [[microservices]]
```

### 7.3 Multi-Agent Development

```
You: "Implement the user authentication system"
AI (Sonnet as PM): [Launches agentic-dev workflow]
    → Discovery Wave: 3 agents explore approaches in parallel
    → Validation Wave: Test security implications
    → Implementation Wave: Coordinated development
    → Each agent's reasoning persisted to memory://
```

**Note**: Multi-agent orchestration requires an MCP client that supports agent dispatch:
- **Claude Code** (Anthropic's CLI)
- **Copilot CLI** (GitHub's command-line interface)
- **aishell** (Open-source MCP shell)
- **Codex** (Multi-agent coordinator)

The PM (typically Sonnet with blue hat) uses sequential thinking to maintain context while dispatching ephemeral sub-agents that burn through their own context windows doing the actual work.

### 7.4 Knowledge Graph Queries

```
You: "What patterns reduce production incidents?"
AI: [Traverses graph from [[incident]] concept]
    → Finds [[error-handling]] mentioned in 80% of solutions
    → [[monitoring]] appears in 65% of preventions
    → [[code-review]] correlates with 40% fewer incidents
    → Returns: Empirical patterns from your codebase history
```

### 7.5 Observing AI Cognition

```
You: "Show me what you've been thinking about"
$ maenifold --tool recentactivity --payload '{"filter":"thinking","limit":3}'

→ session-87234: Debugging race condition (12 thoughts, 3 revisions)
→ session-87199: API design review (8 thoughts, branched at thought 5)
→ workflow-87156: Agile sprint planning (completed, 15 steps)
```

## 8. Real-World Impact

### 8.1 What This Enables

**Multi-day debugging sessions** where the AI remembers every hypothesis:
```
Day 1: "The race condition might be in the auth handler"
Day 3: "Remember when we suspected the auth handler? Let's revisit with new logs"
```

**Systematic code reviews** that build institutional knowledge:
```
Review 1: Creates memory about error handling patterns
Review 5: Surfaces the pattern, suggests team-wide adoption
Review 20: Graph reveals which patterns reduce bugs
```

**Evolving architectures** with full decision history:
```
Q: "Why did we choose PostgreSQL over MongoDB?"
A: [Retrieves original decision, subsequent validations, and current trade-offs]
```

### 8.2 The Compound Effect

Unlike stateless AI that resets each conversation:
- **Week 1**: Basic memory formation
- **Month 1**: Patterns emerge in the graph
- **Month 3**: AI surfaces non-obvious connections
- **Year 1**: Institutional knowledge that would take a team years to document

Every interaction strengthens the graph. Every query can traverse relationships. Every decision builds on previous learning.

間 In the space between sessions, understanding grows

## 9. Real Testing, Real Bugs, Real Confidence

Unlike traditional unit tests that validate mocks against mocks, maenifold employs **multi-agent orchestration testing** that discovers actual issues:

### 9.1 The Hero Demo

Our [comprehensive E2E test](demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md) orchestrated **12 specialized agents** across 4 waves to validate maenifold:

- **Found and fixed a critical bug**: Move operations were losing file extensions - mocks would never catch this
- **Discovered parameter inconsistencies**: minScore filtering wasn't working - only real queries revealed this
- **Validated actual performance**: Real operations against real data, not synthetic benchmarks
- **85% success rate**: Honest assessment, not 100% fake passes

See the [orchestration plan](demo-artifacts/part1-pm-lite/test-matrix-orchestration-plan.md) and [final report](demo-artifacts/part1-pm-lite/hero-demo-final-report.md) for details.

### 9.2 Why This Matters

**Traditional testing with mocks:**
```javascript
// This test always passes but tells you nothing
mockDB.save = jest.fn().mockResolvedValue({ id: 1 });
expect(await service.save(data)).toBe(1); // ✅ Meaningless success
```

**maenifold's approach with real agents:**
```bash
# Agent CORE-01 discovers actual behavior
maenifold --tool movememory --payload '{"source":"test","destination":"verified/test"}'
# Result: File lost .md extension - REAL BUG FOUND
```

### 9.3 Adaptive Test Evolution

When maenifold evolves, tests don't break - they adapt:

1. **Agents discover new capabilities** through tool exploration
2. **PM-lite protocol orchestrates** comprehensive coverage automatically
3. **Sequential thinking sessions** document test reasoning
4. **No test maintenance** - agents test what exists, not what was

The [orchestration session](demo-artifacts/part1-pm-lite/orchestration-session.md) shows how agents coordinate to achieve complete coverage without predefined test cases.

### 9.4 Test Philosophy

Following Ma Protocol's **NO FAKE TESTS** principle:
- Real SQLite, not mocks
- Real file operations, not stubs
- Real vector embeddings, not fixtures
- Real multi-agent coordination, not simulations

∴ Real confidence, not false security

## 10. Technical Principles

### Transparency First
Every operation is inspectable:
- Thinking sessions show all revisions and branches
- Search results include similarity scores
- Graph relationships are queryable SQL
- All content is readable markdown

### Lazy Evaluation
Nothing is pre-computed or pre-structured:
- Graph builds from natural WikiLink usage
- Embeddings generate on demand
- Relationships emerge from repetition
- Structure follows function

### Composable Tools
Each tool does one thing well:
- `WriteMemory` → Creates markdown with WikiLinks
- `SequentialThinking` → Iterative reasoning with revision
- `AssumptionLedger` → Traceable skepticism without auto-inference
- `BuildContext` → Graph traversal from concepts
- `Workflow` → Orchestrates tool composition

### Files as Source of Truth
Not a black box database:
- Direct file access for debugging
- Git-compatible for version control
- Obsidian-compatible for human editing
- Standard markdown for portability

### Complete Observability
Watch AI thinking in real-time:
```
$ maenifold --tool recentactivity --payload '{"filter":"thinking"}'

session-1758434799362 (sequential)
  Modified: 2025-09-20 23:07
  Thoughts: 4
  Status: completed
  First: "Analyzing hybrid search capabilities..."
  Last: "README should be confident but not hyperbolic..."
```

Every reasoning session, every revision, every branch—all queryable, all transparent.

## 11. Project Structure

```
~/maenifold/
├── memory/           # Your markdown memories with WikiLinks
├── memory.db         # Knowledge graph and vector embeddings
└── assets/           # Workflows and system resources
```

## 12. Documentation

- [Demo Artifacts](demo-artifacts/) - Real-world testing with multi-agent orchestration

## 13. License & Attribution

**License**: MIT

Sequential Thinking implementation inspired by [MCP Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking) - enhanced with lazy graph construction and multi-agent collaboration.

---

*maenifold - Test-time reasoning that compounds.*