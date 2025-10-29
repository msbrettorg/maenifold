# Product Requirements Document: Use Cases Documentation
## "From Installation to Implementation"

**Version**: 1.0
**Date**: 2025-01-29
**Author**: Claude Code
**Status**: Draft
**Project**: Maenifold Use Cases Expansion

---

## Executive Summary

This PRD outlines the expansion of maenifold's Get Started section to include comprehensive use-case documentation. While the current `/start` page covers installation and basic configuration, users need practical, scenario-driven guides showing how to apply maenifold in real-world contexts. This expansion adds 4 detailed use-case pages demonstrating integration patterns, workflows, and best practices across different tools and scenarios.

### Vision Statement
> "Show developers exactly how maenifold solves their specific problems—from knowledge management with Obsidian to multi-agent product team orchestration—with practical, copy-paste examples and real demo artifacts."

---

## 1. Goals & Objectives

### Primary Goals
1. **Reduce Time-to-Value**: Get users from installation to productive use in < 15 minutes
2. **Showcase Integration Patterns**: Demonstrate maenifold working with popular tools (Claude Code, Codex, Obsidian, GitHub Copilot)
3. **Enable Self-Service**: Provide complete, actionable documentation requiring no additional support
4. **Highlight Differentiation**: Show unique capabilities (multi-agent orchestration, persistent knowledge graph)

### Success Metrics
- **Adoption**: 40%+ of new users complete at least one use case walkthrough
- **Clarity**: < 5% of users require additional documentation/support after reading
- **Completion**: Users can reproduce examples within 30 minutes
- **Feedback**: "This helped me understand how to use maenifold" > 80% positive

---

## 2. User Stories

### Primary Persona: Knowledge Worker / Researcher

**US-01: Obsidian Integration**
**AS A** knowledge worker using Obsidian
**I WANT TO** integrate maenifold with my existing Obsidian vault
**SO THAT** my AI agents can read/write notes and link concepts across sessions

**Acceptance Criteria**:
- Clear setup instructions for pointing MAENIFOLD_ROOT to Obsidian vault
- Example of AI writing a memory that appears in Obsidian
- Example of searching memories from Claude Code
- Explanation of WikiLink compatibility

---

### Secondary Persona: FinOps Practitioner

**US-02: Financial Reporting Automation**
**AS A** FinOps practitioner
**I WANT TO** use maenifold with GitHub Copilot to generate financial reports
**SO THAT** I can leverage pre-built workflows and domain-specific roles

**Acceptance Criteria**:
- Setup for GitHub Copilot + maenifold
- Example using ftk-workflows
- Demo of ftk-agent, finops-practitioner, and cfo roles
- Link to real demo artifacts (GPT-A, GPT-B, GPT-C, SONNET-A)
- Example output showing report generation

---

### Tertiary Persona: Software Developer

**US-03: Persistent Dev Context with Codex**
**AS A** developer using Codex
**I WANT TO** use maenifold as a persistent memory bank
**SO THAT** my context and decisions persist across Codex sessions

**Acceptance Criteria**:
- Codex configuration with maenifold
- Example of memory structure (projectbrief.md, activeContext.md, progress.md)
- Workflow showing multi-day feature development
- Demo of RecentActivity resuming previous work
- Reference to swe.md agent instructions

---

### Quaternary Persona: Engineering Manager / Product Lead

**US-04: Multi-Agent Product Team Orchestration**
**AS AN** engineering manager
**I WANT TO** orchestrate multiple Claude Code agents as a product team
**SO THAT** I can simulate PM, engineers, QA roles sharing knowledge

**Acceptance Criteria**:
- Setup for multi-agent orchestration
- Reference to "hero demo" (68-minute, 25-agent orchestration)
- Example of PM-lite protocol (12 agents, 4 waves)
- Example of Agentic-SLC workflow (7 agents, bug fixes)
- Link to demo artifacts showing real results
- Explanation of shared knowledge graph

---

## 3. Content Specifications

### 3.1 Use Case Page Template

Each use case page follows this structure:

```markdown
# [Use Case Title]

## Overview
- What problem does this solve?
- Who is this for?
- What will you learn?

## Prerequisites
- Required tools
- Required knowledge/experience
- Time estimate

## Setup
1. Installation steps
2. Configuration
3. Verification

## Step-by-Step Walkthrough
1. Initial setup
2. First example
3. Advanced patterns
4. Common workflows

## Example Output
- Screenshots
- Code samples
- Demo artifacts

## Common Pitfalls
- Known issues
- Troubleshooting
- FAQs

## Next Steps
- Related use cases
- Advanced topics
- Community resources
```

---

### 3.2 UC1: Build Knowledge Foundation (Seed the Graph)

**Page Route**: `/use-cases/knowledge-work`

**Hero Section**:
- Title: "Knowledge Building: Research Workflows + Obsidian Navigation"
- Subtitle: "Use AI research workflows to build rich knowledge graphs, then navigate visually in Obsidian"
- Visual: Diagram showing Research Workflows → memory:// [[WikiLinks]] → Obsidian Graph View

**Prerequisites**:
- Claude Code installed (or compatible MCP client)
- Obsidian installed (optional, for visual navigation)
- Topic to research (e.g., FinOps Framework, system architecture)

**Setup Steps**:
1. Install maenifold: `npm install -g @ma-collective/maenifold`
2. Configure Claude Code with maenifold MCP server
3. Optional: Set MAENIFOLD_ROOT to Obsidian vault for visual navigation
4. Verify with: "Use agentic-research workflow to research [topic]"

**Walkthrough Examples**:

**Example 1: Single-Agent Deep Research (agentic-research)**
```markdown
**Scenario**: Build comprehensive knowledge about FinOps Framework to prepare for UC2.

**Claude Code Prompt**:
"Use agentic-research workflow to research the FinOps Framework"

**What AI Does** (11-step workflow):
1. Research Initiation: Creates research plan with [[coverage-vector]]
2. Knowledge Baseline: Searches memory for existing [[FinOps]] knowledge
3. HyDE Query Expansion: Generates hypothetical documents for semantic search
4. Information Gathering: Executes web research, creates [[cost-optimization]] [[commitment-strategy]] concepts
5. Topic Discovery: Identifies emergent themes like [[rate-optimization]]
6. Multi-Perspective Analysis: Analyzes from analyst, practitioner, CFO viewpoints
7. Synthesis & Reflexion: Six thinking hats quality review
8. Knowledge Integration: Creates memory://research/finops-framework/ with [[WikiLinks]]
9. Information Gain: Validates 20%+ new knowledge gained
10. Research Deliverable: Comprehensive report with findings
11. Quality Validation: Scores 7/10+ research quality

**Output**:
- 10+ memory files at memory://research/finops-framework/
- 50+ [[concepts]] in knowledge graph
- Ready for UC2 domain extensibility

**Why This Matters**: You've seeded the graph with domain knowledge that AI will use to create custom roles.
```

**Example 2: Multi-Agent Think Tank (think-tank)**
```markdown
**Scenario**: Complex research requiring parallel agent investigation.

**Claude Code Prompt**:
"Use think-tank workflow to research cloud financial optimization"

**What AI Does** (4 waves, 15-20 agents):
- **Wave 1: Domain Scoping** (4 parallel agents)
  - Agent 1: Maps [[knowledge-landscape]] and [[domains]]
  - Agent 2: Scans [[academic-literature]] and [[seminal-works]]
  - Agent 3: Identifies [[emerging-patterns]] and [[trends]]
  - Agent 4: Finds [[knowledge-gaps]] and [[opportunities]]

- **Wave 2: Deep Dive** (3-5 parallel agents per domain)
  - Each uses full agentic-research workflow
  - 20+ [[WikiLinks]] per domain
  - Achieves 30%+ information gain

- **Wave 3: Cross-Domain Synthesis** (4 parallel agents)
  - Finds [[interdisciplinary-connections]]
  - Identifies [[emergent-patterns]]
  - Reconciles contradictions
  - Generates [[novel-insights]]

- **Wave 4: Peer Review** (4 parallel agents)
  - Validates methodology (7/10+ score)
  - Assesses evidence quality
  - Verifies logic chains
  - Confirms 80%+ coverage

**Output**:
- 40+ memory files with comprehensive domain knowledge
- 100+ [[concepts]] in interconnected graph
- Multi-perspective synthesis ready for domain work

**Why This Matters**: Parallel research accelerates knowledge building for complex domains.
```

**Example 3: Obsidian Visual Navigation**
```markdown
**Scenario**: After research, navigate your knowledge graph visually.

**Steps**:
1. Open memory folder as Obsidian vault
2. View graph visualization: [[FinOps-Framework]] connects to [[cost-optimization]], [[commitment-strategy]], [[ROI]]
3. Click [[WikiLinks]] to navigate between research files
4. Use backlinks panel to see all mentions of [[Azure]]
5. Search across entire research corpus instantly

**Why Obsidian?**
- Visual graph view shows concept relationships
- Bidirectional links surface unexpected connections
- Markdown-native, no lock-in
- Works alongside maenifold seamlessly

**Why This Matters**: Visual navigation accelerates understanding without leaving your workspace.
```

**Built-In Workflows**:
Two research workflows bundled with maenifold:
- `/assets/workflows/agentic-research.json` - Single-agent, 11 steps, HyDE-enhanced
- `/assets/workflows/think-tank.json` - Multi-agent waves, 15-20 agents, parallel execution

**Code Sample**:
```bash
# Single-agent research via CLI
maenifold --tool Workflow --payload '{
  "workflowId": "agentic-research",
  "response": "Research the FinOps Framework"
}'

# Multi-agent research (requires Claude Code with agent orchestration)
# Use think-tank workflow via Claude Code

# Optional: Visual navigation with Obsidian
export MAENIFOLD_ROOT="~/Documents/ObsidianVault"
```

**Common Pitfalls**:
- ⚠️ **Workflow choice**: Use `agentic-research` for focused research, `think-tank` for complex multi-domain topics
- ⚠️ **Concept density**: Aim for 10-50+ [[WikiLinks]] per research file for rich graph connections
- ⚠️ **Sync after research**: Run `Sync` tool to update graph database after creating memories
- ⚠️ **Obsidian optional**: Graph works without Obsidian—it just provides visual navigation layer
- ⚠️ **WikiLink syntax**: Use `[[concept-name]]` with hyphens for multi-word concepts, not spaces

**Next Steps**:
- **UC2**: Use this knowledge foundation to create domain-specific roles and workflows
- Try `BuildContext` on [[FinOps-Framework]] to explore concept relationships
- Use `Visualize` to generate Mermaid diagrams of knowledge architecture

---

### 3.3 UC2: Domain Extensibility (AI Creates Its Own Expertise)

**Page Route**: `/use-cases/finops`

**Hero Section**:
- Title: "Domain Extensibility: AI Generating FinOps Expertise from Research"
- Subtitle: "Watch AI study your UC1 knowledge, create domain roles/workflows, then produce $323K ROI analysis"
- Visual: Diagram showing UC1 Knowledge → role-creation-workflow → Domain Roles → ftk-workflows → SONNET-A Report

**Prerequisites**:
- Completed UC1 (knowledge foundation about FinOps Framework)
- Claude Code or GitHub Copilot with maenifold MCP
- Access to FinOps Hubs (optional, for live data)

**Setup Steps**:
1. Ensure UC1 research complete: memory://research/finops-framework/ exists
2. Built-in workflows available: role-creation-workflow, higher-order-thinking
3. FinOps Toolkit help files and DB schema (if working with live data)
4. maenifold configured as MCP server

**Walkthrough Examples**:

**Example 1: AI Creates Domain Roles from UC1 Knowledge**
```markdown
**Scenario**: AI studies FinOps Framework knowledge from UC1 and creates 3 specialist roles.

**Claude Code Prompt**:
"Use role-creation-workflow. Study my memory://research/finops-framework/ knowledge
and the FinOps Foundation website to create three roles: finops-practitioner, cfo, and ftk-agent."

**What AI Does** (role-creation-workflow):
1. Assumes prompt-engineer role (10/10 constitutional AI expert)
2. Defines role specifications from FinOps Framework
3. Researches domain knowledge (reads UC1 memory files + FinOps.org)
4. Analyzes constitutional requirements (principles, anti-patterns)
5. Designs cognitive architecture (workflows, evaluation criteria)
6. Creates role structure (JSON with motto, principles, approach)
7. Validates against prompt engineering checklists
8. Saves to assets/roles/:
   - finops-practitioner.json (FinOps Framework expert)
   - cfo.json (Executive financial stewardship)
   - ftk-agent.json (KQL query executor for FinOps Hubs)

**Output**:
- 3 production-ready role definitions
- Each role studies FinOps Framework knowledge from UC1
- AI generated domain expertise from external docs

**Why This Matters**: AI creates its own domain knowledge by studying, not pre-programming.
```

**Example 2: AI Designs Domain Workflows Using New Roles**
```markdown
**Scenario**: AI adopts finops-practitioner role to design analysis workflows.

**Claude Code Prompt**:
"Adopt finops-practitioner role. Use higher-order-thinking workflow to design
ftk-query and ftk-analysis workflows for Azure cost analysis."

**What AI Does**:
1. Adopts finops-practitioner (FinOps Framework expertise)
2. Uses higher-order-thinking workflow:
   - Examines thinking processes for workflow design
   - Evaluates cognitive approaches
   - Synthesizes multiple perspectives
   - Designs ftk-query (data collection) and ftk-analysis (strategic reporting)
3. Then adopts ftk-agent role to add correct KQL queries to workflows
4. Saves to assets/workflows/:
   - ftk-query.json (FinOps data collection & optimization)
   - ftk-analysis.json (FinOps strategic analysis & reporting)

**Output**:
- Domain-specific workflows designed by AI
- Query patterns from ftk-agent role
- Strategic frameworks from finops-practitioner role

**Why This Matters**: AI designs workflows by adopting domain expertise roles it created.
```

**Example 3: Execute Workflows → Produce $323K ROI Report**
```markdown
**Scenario**: Run ftk-analysis workflow using all 3 roles to produce executive report.

**Process**:
1. ftk-agent executes KQL queries against FinOps Hubs
   - Collects [[cost-data]], [[commitments]], [[anomalies]]
   - Stores results in memory:// with [[WikiLinks]]

2. finops-practitioner analyzes findings
   - Applies [[FinOps-Framework]] best practices
   - Identifies [[optimization-opportunities]]
   - Calculates [[ROI]] and [[payback-period]]

3. cfo synthesizes executive report
   - Strategic context for board presentation
   - Risk assessment and mitigation
   - Multi-scenario financial projections

**Output** (SONNET-A Report):
- **Annual Savings**: $323,875
- **Implementation Cost**: $50,700
- **ROI**: 638%
- **Payback Period**: 1.9 months
- **Strategic Roadmap**: 18-month transformation to B+ FinOps maturity
- **Current State**: Grade D+ (5th percentile commitment strategy)
- **Target State**: 85th percentile industry performance

**Why This Matters**: $323K savings identified → ROI on setup time achieved immediately.
```

**Demo Artifacts - Available for Inspection**:

Real FinOps analysis outputs (SONNET-A folder):
- Location: Available on website for inspection
- Contents:
  - finops-strategic-report.md (367 lines, executive analysis)
  - executive-summary.md (business impact)
  - implementation-roadmap.md (18-month transformation plan)
  - recommendations.md (actionable optimizations)
  - roi-analysis.json (financial projections)
  - 20+ JSON data files (cost breakdowns, forecasts, anomalies)

**Domain Roles - Available for Inspection**:
- finops-practitioner.json (FinOps Framework principles)
- cfo.json (Executive financial stewardship)
- ftk-agent.json (KQL query executor)

**Domain Workflows - Available for Inspection**:
- ftk-query.json (Data collection methodology)
- ftk-analysis.json (Strategic reporting workflow)

**The Meta-Capability**:
```markdown
This isn't about pre-built FinOps roles. It's about:

1. UC1: AI researches FinOps Framework → builds knowledge graph
2. UC2: AI studies that knowledge → creates domain roles
3. UC2: AI adopts roles → designs workflows
4. UC2: AI executes workflows → produces $323K ROI analysis

**The Innovation**: AI generating domain expertise on demand by studying external docs.
```

**Code Sample**:
```bash
# Create roles from research knowledge
maenifold --tool Workflow --payload '{
  "workflowId": "role-creation-workflow",
  "response": "Create finops-practitioner role from memory://research/finops-framework/"
}'

# Design workflows using new roles
maenifold --tool Workflow --payload '{
  "workflowId": "higher-order-thinking",
  "response": "Design ftk-analysis workflow for Azure cost optimization"
}'

# Execute analysis (requires FinOps Hub access)
maenifold --tool Workflow --payload '{
  "workflowId": "ftk-analysis",
  "response": "Analyze Azure costs for fiscal year 2025"
}'
```

**Common Pitfalls**:
- ⚠️ **UC1 prerequisite**: Must complete UC1 research first—roles need knowledge foundation
- ⚠️ **Role creation time**: Expect 20-30 minutes per role for proper constitutional design
- ⚠️ **Workflow design**: higher-order-thinking requires systematic reasoning, not quick prompts
- ⚠️ **FinOps Hub access**: Live analysis requires Azure MCP Kusto Tools connection
- ⚠️ **Asset modification**: Roles/workflows saved to ~/maenifold/assets/ (user-modifiable)

**ROI Calculation**:
```markdown
**Setup Investment**:
- UC1 Research: 60-90 minutes (one-time)
- Role Creation: 60-90 minutes (3 roles, one-time)
- Workflow Design: 30-45 minutes (one-time)
**Total Setup**: ~3 hours one-time investment

**Output Value**:
- $323K annual savings identified
- 638% ROI
- 1.9-month payback period
- Ongoing analysis capability

**Why Should I Care?**
Configure once, get ongoing multi-perspective financial analysis forever.
Suddenly you have analyst + practitioner + CFO analyzing your spend.
```

**Next Steps**:
- **UC3**: Single-agent dev workflows (benefit without multi-agent complexity)
- Try applying this pattern to different domains (security, compliance, architecture)
- Explore other bundled workflows: agentic-dev, agentic-test, agentic-troubleshooting
- View demo artifacts on website for inspection

---

### 3.4 UC3: Development Memory Bank with Codex

**Page Route**: `/use-cases/dev-work`

**Hero Section**:
- Title: "Codex + maenifold: Persistent Development Context"
- Subtitle: "Never lose context between coding sessions. Your decisions, architecture, and progress persist in a living knowledge graph."
- Visual: Diagram showing Codex → maenifold → Project memory → Future sessions

**Prerequisites**:
- Codex installed
- Active development project
- Basic understanding of project documentation

**Graph-First Reset Protocol**

Because Codex (and all agents) experience memory resets between sessions, maenifold provides a systematic recovery protocol:

1. **Sync** → Ensure graph reflects latest markdown changes
2. **RecentActivity** → Identify active sessions and recent documents
3. **SearchMemories** → Find relevant knowledge using hybrid search
4. **BuildContext** → Navigate concept relationships for full context

This protocol ensures every session starts with complete context from the knowledge graph, not from scratch.

**Core Workflows**

**Plan Mode**: Discovery and planning
```
Start → Sync → RecentActivity → SearchMemories → BuildContext/Visualize
→ Read surfaced files → Create plan → Document with references
```

**Act Mode**: Implementation and execution
```
Start → Resume thinking session → Execute work → WriteMemory/EditMemory
→ Sync → Update activeContext/progress → Summarize with references
```

These workflows are embedded in the `swe.md` agent instructions and automatically guide Codex through systematic development.

**Setup Steps**:
1. Install maenifold: `npm install -g @ma-collective/maenifold`
2. Configure Codex: Add to `~/.codex/config.toml`:
   ```toml
   [mcp_servers.maenifold]
   type = "stdio"
   command = "maenifold"
   args = ["--mcp"]
   startup_timeout_sec = 120
   tool_timeout_sec = 600
   env = { MAENIFOLD_ROOT = "~/maenifold" }
   ```
3. Optionally: Copy swe.md agent instructions to `~/.codex/agents/swe.md`
4. Verify: `codex chat "List maenifold MCP tools"`

**Walkthrough Examples**:

**Example 1: Day 1 - Starting New Feature**
```markdown
**Scenario**: You're starting work on a new authentication feature.

**Codex Session**:
"I'm starting work on JWT authentication. Write a project brief and
initial architecture decisions to memory."

**What maenifold Does**:
1. Creates `memory://projects/myapp/projectbrief.md`
   - Scope: JWT authentication feature
   - Stakeholders: Backend team
   - Constraints: Must support refresh tokens

2. Creates `memory://projects/myapp/systemPatterns.md`
   - Architecture: JWT stored in httpOnly cookies
   - Security: HMAC SHA-256 signing
   - Refresh strategy: Sliding window pattern

3. Creates `memory://projects/myapp/activeContext.md`
   - Current: Implementing token generation
   - Next: Add refresh token rotation
   - Blockers: Need to decide token expiry time

All files contain [[JWT]], [[authentication]], [[security]] WikiLinks.
```

**Example 2: Day 2 - Resume Work**
```markdown
**Scenario**: You're back the next day. Codex memory is reset.

**Codex Session**:
"What was I working on yesterday? Show recent activity."

**maenifold Response via RecentActivity**:
```
Recent activity (last 24 hours):
1. memory://projects/myapp/activeContext.md (8 hours ago)
   - Current: Implementing token generation
   - Next: Add refresh token rotation

2. memory://projects/myapp/systemPatterns.md (8 hours ago)
   - JWT architecture decisions documented

3. memory://thinking/session-1234567890.md (8 hours ago)
   - Sequential thinking: Security considerations
```

**Codex automatically**:
- Reads activeContext.md
- Has full context of yesterday's decisions
- Knows exactly where to continue
- No need to re-explain the feature
```

**Example 3: Multi-Day Feature Development**
```markdown
**Day 1**: Architecture & planning
- Write projectbrief.md, systemPatterns.md
- Document [[security-requirements]]

**Day 2**: Implementation starts
- Update activeContext.md with progress
- Write memories about [[implementation-challenges]]
- Link to relevant [[code-patterns]]

**Day 3**: Testing & refinement
- SearchMemories for [[security-requirements]]
- Verify all requirements met
- Update progress.md with completion status

**Day 4**: Code review prep
- BuildContext around [[JWT]] concept
- Generate summary of all related decisions
- Create review checklist from memory

**Result**: Complete development history, every decision documented,
full traceability from requirements to implementation.
```

**Reference: swe.md Agent Instructions**

Codex can be configured with the Software Engineer agent instructions at:
`/Users/brett/.codex/agents/swe.md`

Key features of swe.md:
- Graph-First Reset Protocol (Sync → RecentActivity → SearchMemories)
- memory:// structure guidelines
- Core files: projectbrief, activeContext, systemPatterns, techContext, progress
- Automatic [[concept]] linking best practices

**Example swe.md usage**:
```bash
# Codex uses swe.md instructions automatically when configured
codex chat "Start new project for user authentication system"

# Codex will:
1. Run Sync to ensure graph is current
2. Create memory://projects/auth-system/ structure
3. Write projectbrief.md with [[authentication]] [[security]] concepts
4. Set up activeContext.md for session tracking
```

**Project Memory Structure**:
```
memory://
└── projects/
    └── myapp/
        ├── projectbrief.md        # Foundation: scope, stakeholders, constraints
        ├── productContext.md      # User needs, desired outcomes
        ├── systemPatterns.md      # Architecture, integrations, invariants
        ├── techContext.md         # Tooling, environment, dependencies
        ├── activeContext.md       # Current priorities, next steps, blockers
        └── progress.md            # Status log, milestone tracking
```

**Common Pitfalls**:
- ⚠️ **Forgetting to update activeContext**: Update after each session or you'll lose "current state"
- ⚠️ **Not using [[concepts]]**: WikiLinks are how maenifold links knowledge—use liberally!
- ⚠️ **Skipping Sync**: Run Sync after writing memories so they're searchable
- ⚠️ **Over-documenting**: Focus on decisions and "why", not every code change
- ⚠️ **Ignoring Graph-First Reset Protocol**: Always start sessions with Sync → RecentActivity → SearchMemories to rebuild full context
- ⚠️ **Reading files blindly**: Use SearchMemories and BuildContext to navigate the graph, don't manually read every file
- ⚠️ **Not tracking assumptions**: Use Assumption Ledger for architectural decisions with uncertainty—link to activeContext/progress for traceability

**Advanced Patterns**:

**Pattern 1: Assumption Tracking**
```bash
# When making architectural decisions with uncertainty
codex chat "We're assuming users want single sign-on. Track this assumption."

# maenifold creates assumption ledger entry
# Later, when assumption is validated/invalidated:
codex chat "Update assumption: SSO requirement confirmed by product team"
```

**Pattern 2: Cross-Project Learning**
```bash
# Reuse patterns from previous projects
codex chat "Search memories for [[authentication]] patterns across all projects"

# maenifold returns authentication approaches from ALL projects
# Codex suggests best practices based on past experience
```

**Pattern 3: Team Knowledge Sharing**
```bash
# Multiple developers sharing same MAENIFOLD_ROOT
export MAENIFOLD_ROOT="/shared/team-knowledge"

# Each developer's Codex writes to shared graph
# All team members benefit from collective knowledge
# Search finds insights from entire team's work
```

**Next Steps**:
- Explore SequentialThinking for complex problem-solving
- Try Workflows for systematic development methodologies
- See multi-agent orchestration for team simulation

---

### 3.5 UC4: Multi-Agent Product Team Orchestration

**Page Route**: `/use-cases/product-team`

**Hero Section**:
- Title: "Claude Code + Subagents: Orchestrate an Entire Product Team"
- Subtitle: "Deploy 25+ agents in parallel waves, simulating PM, engineers, QA, and red team—all sharing a single knowledge graph."
- Visual: Diagram showing orchestration hub → agent waves → shared knowledge graph

**Prerequisites**:
- Claude Code installed and configured
- Understanding of multi-agent patterns
- Optional: Read demo artifacts first for context

**The Hero Demo: 68 Minutes, 25 Agents, Real Production Work**

On January 21, 2025, we ran a comprehensive demonstration of multi-agent orchestration using maenifold. This wasn't a toy example—it was real production work that discovered and fixed critical bugs.

**Demo Stats**:
- **Duration**: 68 minutes total
- **Agents Deployed**: 25 agents across 2 phases
- **Issues Fixed**: 4 bugs/features (SRCH-004, MEM-009, GRPH-009, critical move bug)
- **Test Coverage**: 2,031 lines of tests added
- **Knowledge Graph Impact**: 171,506 new concept relations

**Part 1: PM-lite Protocol (Discovery & Testing)**
- **Duration**: 28 minutes
- **Agents**: 12 agents across 4 waves
  - Wave 1: 3 agents (core functionality)
  - Wave 2: 4 agents (integration testing)
  - Wave 3: 3 agents (edge cases)
  - Wave 4: 2 agents (verification)
- **Coordination**: Sequential thinking only (no rigid workflow)
- **Result**: Discovered critical move operation bug, 85% test success rate

**Part 2: Agentic-SLC Workflow (Issue Remediation)**
- **Duration**: 40 minutes
- **Agents**: 13 agents total
  - 7 implementation agents across 3 waves
  - 6 discovery/validation agents
- **Coordination**: 17-step workflow with quality gates
- **Result**: Fixed 3 issues, comprehensive regression tests added

**Setup for Multi-Agent Orchestration**:

1. **Single Claude Code instance as orchestrator**:
   ```bash
   # Set shared knowledge location
   export MAENIFOLD_ROOT="~/project-sprint"

   # Start Claude Code
   claude-code
   ```

2. **Orchestrator creates plan**:
   ```
   "Create a sequential thinking session for orchestrating test coverage.
   Plan agent waves: discovery → implementation → verification → red team."
   ```

3. **Deploy agent waves** (using Claude Code subagents):
   ```
   "Launch 4 parallel subagents:
   - TST-001: Test API endpoints
   - TST-002: Test CLI parity
   - TST-003: Test error handling
   - TST-004: Performance benchmarks

   Each agent should write findings to memory with [[test-result]] tags."
   ```

4. **Agents work in parallel**, writing to shared graph:
   - Each agent has own sequential thinking session
   - All write to same memory:// location
   - Knowledge graph automatically links related concepts
   - Orchestrator monitors progress via RecentActivity

5. **Orchestrator synthesizes results**:
   ```
   "Search memories for [[test-result]] from the last 10 minutes.
   Build context and create summary report."
   ```

**Walkthrough: Simplified PM-lite Demo**

Let's recreate a scaled-down version of the demo (4 agents instead of 12):

**Step 1: Orchestrator Creates Plan**
```markdown
**Claude Code Prompt** (you, as PM):
"I need to test our search functionality. Create a plan for 4 test agents:
- Agent A: Test hybrid search
- Agent B: Test semantic search
- Agent C: Test full-text search
- Agent D: Test edge cases

Create a sequential thinking session to track orchestration."
```

**Step 2: Launch Wave 1 (Parallel Testing)**
```markdown
**Claude Code Prompt**:
"Launch 4 subagents in parallel. Each should:
1. Run assigned tests
2. Write results to memory with [[test-result]] tag
3. Document any bugs found with [[bug]] tag

Use these agent names: TST-A, TST-B, TST-C, TST-D"
```

**What Happens**:
- Claude Code spawns 4 parallel agents (via Task tool)
- Each agent runs independently
- Each writes to memory://testing/ folder
- All agents share the knowledge graph
- You see 4 progress indicators running simultaneously

**Step 3: Monitor Progress**
```markdown
**Claude Code Prompt**:
"Check recent activity. Show me what each agent has completed."

**maenifold Response**:
```
Recent activity:
- TST-A: Completed hybrid search tests (2 min ago) ✅
- TST-B: Found edge case bug in semantic search (1 min ago) ⚠️
- TST-C: Full-text tests passing (1 min ago) ✅
- TST-D: Edge case testing in progress...
```
```

**Step 4: Synthesize Results**
```markdown
**Claude Code Prompt**:
"Search memories for [[test-result]] and [[bug]].
Build context and create summary report."

**maenifold Automatically**:
1. Searches all agent memories
2. Finds test results + bug reports
3. Builds concept graph showing relationships
4. Generates comprehensive report

**Report Includes**:
- Test coverage matrix
- Pass/fail summary
- Bug details with links to agent reports
- Recommendations for next steps
```

**Real Demo Artifacts**

We've preserved the complete demo artifacts showing every step:

**Location**: `/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts`

**Part 1: PM-lite Protocol** (`part1-pm-lite/`)
- `orchestration-session.md`: Real-time orchestration thoughts (session-1758470366887)
- `test-matrix-orchestration-plan.md`: 50+ test cases, 4-wave architecture
- `E2E_TEST_REPORT.md`: Final results showing 85% success rate
- `demo-final-report.md`: Comprehensive report from VRFY-02 agent

**Part 2: Agentic-SLC Workflow** (`part2-agentic-slc/`)
- `agentic-slc-thinking-session.md`: Sprint progress (session-1758474798193)
- `agentic-slc-workflow-session.md`: Complete 17-step workflow (includes embedded RTM.md content)
- `RTM.md`: Requirements traceability matrix (27 atomic items) - embedded in workflow session, not standalone
- `sprint-specifications.md`: Detailed specs with line numbers
- `implementation-plan.md`: 7 agents, 3 waves, 22 tasks
- `code-justification-report.md`: Justification for every line of code
- `sprint-retrospective.md`: Learnings and metrics

**Browse the artifacts**:
```bash
cd /Users/brett/src/ma-collective/maenifold/assets/demo-artifacts
cat README.md  # Start here for overview
cd part1-pm-lite
cat orchestration-session.md  # See real orchestration thoughts

# Note: RTM.md content is embedded in part2-agentic-slc/agentic-slc-workflow-session.md
# Look for the RTM section within the workflow session file
```

**Key Patterns from Demo**:

**Pattern 1: Adaptive Wave Deployment**
```markdown
Instead of launching all 12 agents at once, the PM orchestrator:
1. Launched Wave 1 (4 agents): Core functionality tests
2. Monitored results via RecentActivity
3. Discovered critical bug in Wave 1 results
4. Adapted Wave 2 plan based on findings
5. Launched Wave 2 (3 agents): Bug investigation + regression tests
6. Continued adaptive deployment through Waves 3 & 4

**Result**: More efficient than rigid plan, found critical issues early
```

**Pattern 2: Shared Sequential Thinking**
```markdown
All 12 agents contributed to SAME sequential thinking session:
- PM wrote orchestration thoughts
- Agents wrote progress updates
- Verification agents wrote validation results
- Red team wrote security concerns

**Result**: Complete audit trail, single source of truth for entire sprint
```

**Pattern 3: Knowledge Graph Synthesis**
```markdown
After all agents completed work:
- 171,506 new concept relations added to graph
- BuildContext on [[test-coverage]] revealed connections across all agent work
- SearchMemories found patterns humans might miss
- Visualize showed concept clusters and hotspots

**Result**: Machine-discoverable insights from multi-agent collaboration
```

**Orchestration Strategies**:

**Strategy 1: PM-lite (Flexible, Adaptive)**
- **When**: Exploratory work, unclear requirements
- **Coordination**: Sequential thinking session for orchestration thoughts
- **Agent Communication**: Via shared knowledge graph only
- **Pros**: Adaptive, handles emergent complexity
- **Cons**: Requires skilled PM, less structured

**Strategy 2: Workflow-Driven (Structured, Repeatable)**
- **When**: Well-defined process, quality gates required
- **Coordination**: Predefined workflow (e.g., Agentic-SLC)
- **Agent Communication**: Workflow steps + knowledge graph
- **Pros**: Repeatable, traceable, enforces quality
- **Cons**: Less flexible, requires upfront planning

**Strategy 3: Hybrid (Best of Both)**
- **When**: Complex projects with both structure and uncertainty
- **Coordination**: Workflow for process, sequential thinking for adaptation
- **Example from demo**: Part 2 used Agentic-SLC workflow but PM made adaptive decisions within workflow steps
- **Pros**: Structure + flexibility
- **Cons**: Most complex to orchestrate

**Common Pitfalls**:
- ⚠️ **Too many parallel agents**: Start with 2-4, not 25. Scale up as you learn
- ⚠️ **Forgetting to sync**: Run Sync after agent waves so knowledge is discoverable
- ⚠️ **No orchestration plan**: Even flexible PM-lite needs upfront wave planning
- ⚠️ **Ignoring agent outputs**: Use RecentActivity and SearchMemories to monitor progress
- ⚠️ **Over-orchestrating**: Let agents work independently, don't micromanage

**Advanced: Reproducing the Full Demo**

Want to run the full 25-agent demo yourself?

1. **Prerequisites**:
   - Claude Code with subagent support
   - MAENIFOLD_ROOT set to empty directory
   - 1-2 hours of runtime
   - Real codebase to test (or use maenifold itself)

2. **Part 1: PM-lite Discovery** (follow `part1-pm-lite/test-matrix-orchestration-plan.md`):
   - Define 50+ test cases
   - Plan 4 waves of agents
   - Launch waves sequentially
   - Monitor with RecentActivity
   - Synthesize results

3. **Part 2: Agentic-SLC Remediation** (follow `part2-agentic-slc/RTM.md`):
   - Create requirements traceability matrix
   - Write detailed specifications
   - Plan agent waves
   - Launch implementation agents
   - Launch red team validation
   - Synthesize and review

4. **Analysis**:
   - Run Knowledge Graph Impact Analysis (see `sprint-knowledge-graph-impact-analysis.md`)
   - Measure concept relations added
   - Verify knowledge retrieval works

**Expected Results**:
- Complete test coverage
- Bugs discovered and fixed
- Comprehensive documentation
- Massive knowledge graph growth
- Reusable patterns for future sprints

**Next Steps**:
- Start small: 2-4 agents, simple task
- Read full demo artifacts for patterns
- Experiment with PM-lite vs Workflow coordination
- Scale up as you learn orchestration techniques
- Join community to share your orchestration patterns

---

## 4. Technical Requirements

### 4.1 Page Structure

**Route Structure**:
```
/use-cases/
├── knowledge-work/page.tsx
├── finops/page.tsx
├── dev-work/page.tsx
└── product-team/page.tsx
```

**Shared Components**:
```
components/
├── UseCaseCard.tsx          # Card for listing use cases
├── CodeExample.tsx          # Syntax-highlighted code blocks
├── SetupSteps.tsx           # Numbered setup instructions
├── DemoArtifactLink.tsx     # Links to demo artifacts with previews
└── PitfallWarning.tsx       # Warning callouts for common issues
```

### 4.2 Content Requirements

**Each Use Case Page Must Include**:
1. ✅ Hero section with title, subtitle, visual
2. ✅ Prerequisites list
3. ✅ Step-by-step setup instructions
4. ✅ 3+ practical examples with code
5. ✅ Common pitfalls section
6. ✅ Next steps with related links

**Accessibility Requirements**:
- WCAG 2.1 Level AA compliance
- Code blocks with proper ARIA labels
- Skip navigation for long pages
- Keyboard-navigable examples

**Performance Requirements**:
- Page load < 2 seconds
- Lazy load demo artifact previews
- Optimize images/diagrams
- Lighthouse score 95+

### 4.3 Asset Requirements

**Demo Artifacts**:
- Link to external locations (no inline)
- Provide context/description for each artifact
- Downloadable where appropriate

**Code Examples**:
- Syntax highlighting (use Prism.js or similar)
- Copy-to-clipboard buttons
- Verify all examples work (CI test)

**Diagrams**:
- SVG format for scalability
- Accessible alt text
- Dark mode variants

### 4.4 Integration Requirements

**Navigation Updates**:
- Add "Use Cases" to main nav (dropdown or page)
- Update `/start` page with use case cards
- Add breadcrumbs to use case pages

**Cross-Linking**:
- Link between related use cases
- Link from use cases back to tools
- Link to docs/architecture where relevant

---

## 5. Implementation Plan

### Phase 1: Structure & Foundation

**Tasks**:
1. Create `/use-cases/[slug]/page.tsx` route structure
2. Build shared components (UseCaseCard, CodeExample, etc.)
3. Update main nav to include "Use Cases"
4. Create placeholder pages for all 4 use cases

**Deliverables**:
- Working route structure
- Reusable components
- Navigation updated

**Validation**:
- [ ] All routes accessible
- [ ] Navigation works on all pages
- [ ] Components render correctly

---

### Phase 2: Content Creation - UC3 & UC1

**Why These First**:
- UC3 (Codex): Have complete swe.md reference
- UC1 (Obsidian): Straightforward setup, clear value prop

**Tasks**:
1. Write UC3 (Codex) content
   - Reference swe.md instructions
   - Create 3 practical examples
   - Document project memory structure
2. Write UC1 (Obsidian) content
   - Explain MAENIFOLD_ROOT setup
   - Show WikiLink compatibility
   - Demo research workflow

**Deliverables**:
- UC3 page complete with examples
- UC1 page complete with examples
- All code examples tested and verified

**Validation**:
- [ ] UC3: Can follow instructions and reproduce setup
- [ ] UC1: Can integrate Obsidian + Claude Code in < 15 min
- [ ] All code examples run without errors

---

### Phase 3: Content Creation - UC2 & UC4

**Tasks**:
1. Write UC2 (FinOps) content
   - Link to demo artifacts in `/Users/brett/src/finops-worktrees/extras/research`
   - Explain ftk-workflows and roles
   - Show report generation example
2. Write UC4 (Product Team) content
   - Reference demo artifacts in `/assets/demo-artifacts`
   - Explain PM-lite vs Workflow orchestration
   - Provide scaled-down demo (4 agents instead of 25)

**Deliverables**:
- UC2 page complete with FinOps examples
- UC4 page complete with orchestration guide
- Demo artifact links and context

**Validation**:
- [ ] UC2: Can set up GitHub Copilot + FinOps in < 30 min
- [ ] UC4: Understand orchestration patterns
- [ ] Demo artifacts accessible and understandable

---

### Phase 4: Polish & Integration

**Tasks**:
1. Add "Use Cases" section to `/start` page
   - 4 cards with links to use case pages
   - Brief description of each
   - Visual icons/illustrations
2. Create diagrams for each use case
   - Obsidian ↔ maenifold ↔ Claude Code
   - GitHub Copilot → ftk-workflows → Reports
   - Codex → Project memory → Sessions
   - Orchestrator → Agent waves → Shared graph
3. Add cross-links between pages
4. Final copy editing and testing

**Deliverables**:
- Updated `/start` page
- All diagrams created
- Cross-linking complete

**Validation**:
- [ ] User can discover all use cases from `/start`
- [ ] Navigation flows naturally
- [ ] All examples verified to work

---

### Phase 5: Testing & Validation

**Tasks**:
1. User testing with 5+ external users
2. Gather feedback on clarity and completeness
3. Fix any issues discovered
4. Run accessibility audit
5. Performance testing

**Success Criteria**:
- [ ] 4/5 users can complete UC3 setup without help
- [ ] 4/5 users understand UC4 orchestration concepts
- [ ] Lighthouse score 95+ on all pages
- [ ] WCAG AA compliance

---

## 6. Success Criteria

### Must Have (MVP)

**Content Completeness**:
- ✅ All 4 use case pages published
- ✅ Each page has 3+ practical examples
- ✅ Setup instructions tested and working
- ✅ Demo artifacts linked and accessible

**User Experience**:
- ✅ User can find use cases from landing page
- ✅ Code examples have copy-to-clipboard
- ✅ Page load < 2 seconds
- ✅ Mobile responsive

**Quality**:
- ✅ All code examples verified to work
- ✅ No broken links
- ✅ Lighthouse score 95+
- ✅ WCAG AA compliant

---

### Should Have

**Enhanced Examples**:
- ⭕ Video walkthrough for UC4 (orchestration demo)
- ⭕ Interactive code playground for examples
- ⭕ Downloadable starter templates

**Community Features**:
- ⭕ "Share your setup" section
- ⭕ Link to community examples/patterns
- ⭕ Discussion forum integration

---

### Nice to Have

**Advanced Content**:
- ⭕ Advanced orchestration patterns page
- ⭕ Troubleshooting guide with common errors
- ⭕ Performance tuning guide
- ⭕ Security best practices

**Interactive Elements**:
- ⭕ Live demo environment (sandboxed)
- ⭕ Progress tracking for multi-step walkthroughs

---

## 7. Dependencies & Constraints

### Technical Dependencies
- Next.js 16.0.0 (already in place)
- Tailwind CSS 4.1.16 (already in place)
- Syntax highlighter (Prism.js or similar) - **NEW**
- Demo artifacts must remain accessible at specified paths

### External Dependencies
- `/Users/brett/src/finops-worktrees/extras/research` (FinOps artifacts)
- `/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts` (Hero demo)
- `/Users/brett/.codex/agents/swe.md` (Codex instructions)

### Constraints
1. **Content Accuracy**: All examples must be tested and working
2. **Demo Artifacts**: Cannot modify original artifacts, only link/reference
3. **Maintenance**: Examples must be updated when maenifold tools change
4. **Performance**: Must not slow down site (lazy load where possible)

---

## 8. Risk Mitigation

**Risk 1: Examples Become Outdated**
- **Mitigation**: Add "Last verified" date to each example
- **Mitigation**: CI test that runs code examples monthly
- **Mitigation**: Version pin examples (e.g., "maenifold v1.0.0")

**Risk 2: Demo Artifacts Moved/Deleted**
- **Mitigation**: Copy critical artifacts to site/public/demo-artifacts
- **Mitigation**: Provide context even if artifacts unavailable
- **Mitigation**: Screenshots as backup

**Risk 3: Users Can't Reproduce Examples**
- **Mitigation**: Thorough testing with external users
- **Mitigation**: Troubleshooting section on each page
- **Mitigation**: Link to community support

**Risk 4: Overwhelming Complexity (UC4)**
- **Mitigation**: Provide simplified version first (4 agents)
- **Mitigation**: Clear progression: simple → intermediate → advanced
- **Mitigation**: Optional "deep dive" sections, not required reading

---

## 9. Metrics & Tracking

### Engagement Metrics
- Page views for each use case
- Scroll depth (target: 80%+ reach "Next Steps")
- Click-through to demo artifacts

### Success Metrics
- % of users completing setup (via form or analytics)
- Support tickets mentioning use cases (target: < 5% need help)
- Community contributions (users sharing their setups)

### Quality Metrics
- Lighthouse scores (all 95+)
- Broken link count (0)
- Code example success rate (100% tested monthly)
- Accessibility issues (0 critical, 0 serious)

---

## 10. Appendix

### A. Reference Materials

**Existing Documentation**:
- `/Users/brett/.codex/agents/swe.md` - Codex agent instructions
- `/Users/brett/src/ma-collective/maenifold/docs/PRD-landing-page-redesign.md` - Landing page PRD
- `/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts/README.md` - Demo overview

**Demo Artifacts**:
- FinOps: `/Users/brett/src/finops-worktrees/extras/research`
- Hero Demo: `/Users/brett/src/ma-collective/maenifold/assets/demo-artifacts`

**Related Sites**:
- Obsidian: https://obsidian.md
- GitHub Copilot: https://github.com/features/copilot
- Codex: (provide link when available)

---

### B. Content Style Guide

**Tone**:
- Professional but approachable
- Focus on "show, don't tell"
- Assume technical competence but not maenifold expertise

**Code Examples**:
- Always include expected output
- Show both success and common error cases
- Use realistic examples (not "foo", "bar")

**Terminology**:
- "memory://" (not "memory folder" or "knowledge base")
- "WikiLinks" (not "wiki links" or "concept links")
- "knowledge graph" (not "graph database")
- "subagent" (not "sub-agent" or "agent task")

---

### C. Stakeholder Sign-off

**Reviewed By**: [Pending]
**Approved By**: [Pending]
**Date**: [Pending]

---

## Document Control

**Version History**:
- v1.0 (2025-01-29): Initial PRD

**Related Documents**:
- Landing Page PRD: `/docs/PRD-landing-page-redesign.md`
- Sequential Thinking Session: `session-1761705676636`

**Contact**:
For questions or clarifications, refer to project repository issues.

---

**END OF DOCUMENT**
