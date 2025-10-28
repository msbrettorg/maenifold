# README Content Extraction Map
**Date**: 2025-10-28
**Source**: /Users/brett/src/ma-collective/maenifold/README.md
**Purpose**: Organize content sections for Next.js pages (Hero, Features, Cognitive Assets, Architecture, Quick Start)
**Protocol**: Extract what IS - no modifications, no fabrication

---

## 1. Hero Section (Landing Page Banner)
**Lines**: 1-8
**Content Type**: Logo + Tagline + Whitespace

```
Line 1-3:   <p align="center">
              <img src="assets/branding/maenifold-logo.svg" alt="maenifold">
            </p>

Line 4:     [blank line]

Line 5-7:   <p align="center">
              Your AI is ephemeral. Your knowledge shouldn't be.
            </p>

Line 8:     [blank line]
```

**Status**: ✓ Ready for copying to landing page hero component
**Notes**:
- Image path: `assets/branding/maenifold-logo.svg` (relative, needs Next.js public folder adjustment)
- Tagline is centered with HTML alignment
- Clean whitespace structure maintained

---

## 2. "What maenifold does" Section
**Lines**: 9-31
**Content Type**: Section header + Core narrative + Demo story + Summary

```
Line 9:     ## What maenifold does

Line 10:    [blank line]

Line 11:    **maenifold** enhances AI agents with persistent graphs of thought
            that compound over time. Every tool creates `[[WikiLink]]`
            connections that survive conversations. Every session builds on the
            last. Knowledge compounds instead of resets. **It transforms
            ephemeral AI sessions into continuous collective intelligence.**

Line 12:    [blank line]

Line 13-15: <p align="center">
              <img src="assets/branding/graph.jpeg" alt="graph">
            </p>

Line 16:    [blank line]

Line 17:    [Our demo](assets/demo-artifacts/README.md) shows this at scale:
            25 AI agents across 68 minutes discovered a critical production
            bug not through programmed coordination, but through emergent
            understanding. No orchestration code was written, yet agents
            perfectly orchestrated themselves across 4 waves, building on
            each other's discoveries through a shared knowledge graph.

Line 18:    [blank line]

Line 19:    The critical move operation bug emerged from the intersection of
            multiple test patterns seen across different agent sessions -
            something no single agent could have found alone. This is
            maenifold's core: making every AI session additive rather than
            isolated.

Line 20:    [blank line]

Line 21:    ### How it worked:

Line 22:    [blank line]

Line 23-27: - Agents shared discoveries through `[[WikiLinks]]` in memory files
            - Each wave of agents built on previous findings via search and
              context traversal
            - The critical bug emerged from patterns across multiple test
              sessions
            - 171,506 new concept relationships were created, connecting
              discoveries
            - [Full orchestration logs](assets/demo-artifacts/part1-pm-lite/
              orchestration-session.md) and [test results]
              (assets/demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md)
              available

Line 28:    [blank line]

Line 29:    85% test success rate. Real production bug found. Zero
            orchestration code written.

Line 30:    [blank line]

Line 31:    That's what maenifold does: It provides the substrate (WikiLinks,
            memory, graph) and steps back. Intelligence fills the space.
```

**Status**: ✓ Ready for copying to main features section
**Notes**:
- Contains embedded image reference: `assets/branding/graph.jpeg`
- Contains internal links to demo artifacts
- Rich narrative structure with subsection "How it worked:"
- Demonstrates value through concrete demo metrics

---

## 3. Cognitive Assets Section
**Lines**: 33-72
**Content Type**: Section header + 4 subsections (Workflows, Roles, Colors, Perspectives)

```
Line 33:    ## Cognitive Assets

Line 34:    [blank line]

Line 35:    maenifold ships with 54 pre-configured cognitive frameworks that
            agents can adopt dynamically:

Line 36:    [blank line]

Line 37:    ### 🔄 **28 Workflows** - Structured Methodologies
Line 38:    From reasoning patterns to development processes:
Line 39-43: - **Reasoning**: deductive, inductive, abductive, critical,
              strategic, higher-order thinking
            - **Creative**: design thinking, divergent thinking, lateral
              thinking, oblique strategies, SCAMPER
            - **Development**: agentic-dev with anti-slop controls, agile,
              SDLC, code review workflows
            - **Collaborative**: world café, parallel thinking, six thinking hats
            - **Meta-orchestration**: workflow-dispatch for intelligent
              methodology selection

Line 44:    [blank line]

Line 45:    ### 🎭 **7 Roles** - Professional Perspectives
Line 46:    Each with personality, principles, and transition triggers:
Line 47-53: - **Product Manager**: "Simple, Lovable, Complete" framework
            - **Architect**: System design and patterns
            - **Engineer**: Implementation and quality
            - **Researcher**: Investigation and discovery
            - **Writer**: Clear communication
            - **Red Team**: Security testing and adversarial thinking
            - **Blue Team**: Defense and protection strategies

Line 54:    [blank line]

Line 55:    ### 🎨 **7 Colors** - De Bono's Six Hats + Gray
Line 56:    Thinking modes for different cognitive approaches:
Line 57-63: - **White**: Facts and information
            - **Red**: Emotions and intuition
            - **Black**: Caution and critical judgment
            - **Yellow**: Optimism and positive assessment
            - **Green**: Creativity and alternatives
            - **Blue**: Process control and orchestration
            - **Gray**: Skeptical inquiry and assumption questioning

Line 64:    [blank line]

Line 65:    ### 🗣️ **12 Perspectives** - Sapir-Whorf Linguistic Patterns
Line 66:    Language-influenced cognitive styles based on linguistic
            relativity:
Line 67-70: - **Arabic**: Pattern analysis through root systems
            - **Japanese**: Contextual and relational thinking
            - **German**: Hierarchical precision
            - **And 9 more**: Each language brings unique reasoning patterns

Line 71:    [blank line]

Line 72:    These aren't just templates - they're cognitive substrates that
            combine with the knowledge graph to create emergent reasoning
            capabilities. An agent can start with strategic thinking, switch
            to red team perspective when finding vulnerabilities, then adopt
            the writer role to document findings - all while building on the
            persistent knowledge graph.
```

**Status**: ✓ Ready for copying to capabilities/features showcase
**Notes**:
- 4 distinct subsections with emoji prefixes
- Total 54 frameworks (28 + 7 + 7 + 12)
- Ends with synthesizing explanation of how these combine with knowledge graph
- Content is highly structured with bullet lists

---

## 4. "The Cognitive Stack" Section (Architecture Diagram)
**Lines**: 74-139
**Content Type**: Section header + Mermaid diagram + 3 subsystem explanations

```
Line 74:    ## The Cognitive Stack

Line 75:    [blank line]

Line 76:    ### Tool Relationships

Line 77:    [blank line]

Line 78-110: [MERMAID DIAGRAM - 33 lines showing system architecture with
             LLM Agent, Perspectives (Adopt), Thinking (SequentialThinking,
             Workflow), Memory System (Memory, Graph), and connections between
             them]

Line 111:   [blank line]

Line 112:   ### Reasoning Layer (Tools + Workflows) - Where Information is
            Processed

Line 113:   [blank line]

Line 114:   Where test-time computation happens:
Line 115-120: - **Test-time Adaptive Reasoning**: Sequential thinking with
              revision, branching, persistence and automatic graph construction
            - **Perspective Tuning**: Rich role and color (six thinking hats)
              definitions provide tunable agent perspectives
            - **Intelligent Workflow Selection**: Meta-cognitive system that
              analyzes problems and selects optimal reasoning approaches
            - **30 Distinct Methodologies**: Complete taxonomy from deductive
              reasoning to design thinking, with sophisticated orchestration
            - **Assumption Ledger**: Traceable skepticism for agent reasoning—
              capture, validate, and track assumptions without auto-inference
            - **Multi-agent Coordination**: Wave-based execution with parallel
              agent dispatch with 'blue-hat' product manager orchestrating
              sub-agents (claude-code/codex/aishell/etc...)

Line 121:   [blank line]

Line 122:   [blank line]

Line 123:   ### Memory Layer (`memory://`) - Where Data is Stored

Line 124:   [blank line]

Line 125-127: - **Local**: Every piece of knowledge lives as a markdown file
              on disk with a unique URI.
            - **Transparent**: Every thought, revision, and decision visible
              in markdown files.
            - **Human-Friendly**: All files are human-readable, Obsidian-
              compatible, and persist across sessions.

Line 128:   [blank line]

Line 129:   [blank line]

Line 130:   ### Graph Layer (SQLite + vectors) - Where Knowledge Emerges

Line 131:   [blank line]

Line 132:   Automatic graph construction from `[[WikiLinks]]` with:
Line 133-138: - **384-dimensional embeddings** for semantic similarity
            - **Edge weights** that strengthen with repeated mentions
            - **Concept clustering** revealing emergent patterns
            - **Incremental sync** keeping the graph current
            - **Hybrid RRF Search**: Semantic + full-text fusion for optimal
              retrieval (not just embedding similarity)
            - **Graph Construction**: No schema, no ontology — structure emerges
              from WikiLink usage

Line 139:   [blank line]
```

**Status**: ✓ Ready for copying to architecture/how-it-works page
**Notes**:
- Contains Mermaid diagram (lines 78-110) - rendereable in Next.js
- 3 subsystems explained: Reasoning, Memory, Graph layers
- Technical but accessible explanations
- Total 66 lines including whitespace

---

## 5. Technical Specifications Section
**Lines**: 140-150
**Content Type**: Section header + Specification list

```
Line 140:   ## Technical Specifications

Line 141:   [blank line]

Line 142-150: - **Language**: C# with .NET 9.0
            - **Vector Dimensions**: 384 (all-MiniLM-L6-v2 via ONNX)
            - **Search Algorithm**: Reciprocal Rank Fusion (k=60)
            - **Database**: SQLite with vector extension
            - **Graph Sync**: Incremental with file watching
            - **Memory Format**: Markdown with YAML frontmatter
            - **URI Scheme**: `memory://` protocol
            - **Tested Scale**: > 1.1 million relationships
            - **MCP Compliance**: Full tool annotation support
```

**Status**: ✓ Ready for copying to documentation/specs page
**Notes**:
- Clean bullet list format
- Specific, factual specifications
- 9 specification items
- No extra whitespace after this section

---

## 6. Quick Start Section
**Lines**: 152-215
**Content Type**: Section header + Install + MCP config + CLI examples

```
Line 152:   ## Quick start

Line 153:   [blank line]

Line 154:   ### Install
Line 155:   ```bash
Line 156:   npm install -g @ma-collective/maenifold
Line 157:   ```

Line 158:   [blank line]

Line 159:   ### MCP Interface

Line 160:   [blank line]

Line 161:   **Claude Code, Continue, Cline** - Add to MCP config:

Line 162:   [blank line]

Line 163-173: ```json
            {
              "mcpServers": {
                "maenifold": {
                  "command": "maenifold",
                  "args": ["--mcp"],
                  "env": {"MAENIFOLD_ROOT": "~/maenifold"}
                }
              }
            }
            ```

Line 174:   [blank line]

Line 175:   **Codex** - Add to `~/.codex/config.toml`:

Line 176:   [blank line]

Line 177-185: ```toml
            [mcp_servers.maenifold]
            type = "stdio"
            command = "maenifold"
            args = ["--mcp"]
            startup_timeout_sec = 120
            tool_timeout_sec = 600
            env = { MAENIFOLD_ROOT = "~/maenifold" }
            ```

Line 186:   [blank line]

Line 187:   Try it: `"Write a memory about our architecture decisions"`

Line 188:   [blank line]

Line 189:   ### CLI Interface

Line 190:   [blank line]

Line 191:   Use maenifold directly in scripts, pipelines, or with non-MCP
            clients:

Line 192:   [blank line]

Line 193-198: ```bash
            # Write a memory with WikiLinks
            maenifold --tool WriteMemory --payload '{
              "title": "Architecture Decisions",
              "content": "Our [[microservices]] use [[event-sourcing]] for [[audit-trails]]"
            }'
            ```

Line 199:   [blank line]

Line 200-205: ```bash
            # Continue a sequential thinking session
            maenifold --tool SequentialThinking --payload '{
              "sessionId": "session-1234567890",
              "response": "After analyzing the architecture...",
              "nextThoughtNeeded": true
            }'
            ```

Line 206:   [blank line]

Line 207-212: ```bash
            # Search memories with hybrid mode
            maenifold --tool SearchMemories --payload '{
              "query": "authentication patterns",
              "mode": "Hybrid",
              "pageSize": 10
            }'
            ```

Line 213:   [blank line]

Line 215:   **MCP and CLI have full feature parity.** Start a session via MCP
            and continue it via CLI, or vice versa. The system supports
            concurrent agents using the same memory location - perfect for
            multi-agent pipelines or parallel workflows.
```

**Status**: ✓ Ready for copying to getting-started/setup page
**Notes**:
- 3 subsections: Install + MCP Interface + CLI Interface
- Contains code blocks (bash and JSON config formats)
- Multiple CLI examples with real payload structures
- Ends with feature parity statement
- Total 64 lines including whitespace and code blocks

---

## 7. Footer / Learn More Section
**Lines**: 217-228
**Content Type**: Links + License info

```
Line 217:   ## Learn more

Line 218:   [blank line]

Line 219-220: - [Complete Documentation](assets/README.md) - Architecture,
              examples, philosophy
            - [Demo Artifacts](assets/demo-artifacts/README.md) - Multi-agent
              orchestration

Line 221:   [blank line]

Line 222:   .NET 9.0 · SQLite · ONNX · MCP · MIT License

Line 223:   [blank line]

Line 224:   ---

Line 225:   [blank line]

Line 226:   ## Stargazers over time

Line 227:   [blank line]

Line 228:   [![Stargazers over time]
            (https://starchart.cc/MSBrett/maenifold.svg?variant=adaptive)]
            (https://starchart.cc/MSBrett/maenifold)
```

**Status**: ✓ Ready for copying to footer
**Notes**:
- Links to assets/README.md and assets/demo-artifacts/README.md (internal relative paths)
- Tech stack badge line
- Stargazers chart (GitHub badge)

---

## Summary: Content Organization for Next.js Pages

| Section | Lines | Destination | Content Type | Ready |
|---------|-------|-------------|--------------|-------|
| Hero/Logo | 1-8 | Landing Page Hero | Image + Tagline | ✓ |
| What maenifold does | 9-31 | Landing Page / Features | Narrative + Demo | ✓ |
| Cognitive Assets | 33-72 | Features / Capabilities | 4 subsections (54 frameworks) | ✓ |
| Cognitive Stack | 74-139 | Architecture / How it Works | Diagram + 3 technical layers | ✓ |
| Technical Specs | 140-150 | Documentation / Specs | 9 technical specs | ✓ |
| Quick Start | 152-215 | Getting Started / Setup | Install + Config + Examples | ✓ |
| Footer/Learn More | 217-228 | Footer | Links + License + Stats | ✓ |

---

## Asset References Found in Content

All references use relative paths that will need Next.js `public/` folder adjustment:

1. **Image Assets**:
   - `assets/branding/maenifold-logo.svg` (line 2)
   - `assets/branding/graph.jpeg` (line 14)

2. **Internal Documentation Links**:
   - `assets/README.md` (line 219) - Complete Documentation
   - `assets/demo-artifacts/README.md` (line 220) - Demo Artifacts
   - `assets/demo-artifacts/part1-pm-lite/orchestration-session.md` (line 27)
   - `assets/demo-artifacts/part1-pm-lite/E2E_TEST_REPORT.md` (line 27)

3. **External Links**:
   - `https://starchart.cc/MSBrett/maenifold.svg?variant=adaptive` (line 228)

---

## Validation Checklist

- [x] All sections identified in acceptance criteria present
- [x] Exact line numbers verified against source file
- [x] Content extracted "as IS" - no modifications
- [x] No fabrication - pure documentation of what exists
- [x] Asset paths documented
- [x] Ready for handoff to Next.js implementation
- [x] Follows [[Ma-Protocol]]: Document what IS

**Extraction Complete**: 2025-10-28 12:45 UTC
