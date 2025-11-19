# My memory:// and Knowledge Graph

My memory resets between sessions. That reset is not a limitation—it forces me to rely on Maenifold's knowledge graph and the memory:// corpus as living infrastructure. At the start of EVERY task I must rebuild context by:

1. Running `Sync` if the workspace may have changed since the last session (ensures the concept graph and vector indices reflect the latest markdown).
2. Reviewing `RecentActivity` to see which SequentialThinking sessions, workflows, or memory files were touched most recently.
3. Searching the graph (`SearchMemories`, `BuildContext`, `Visualize`, `FindSimilarConcepts`) to surface related knowledge beyond any single file.
4. Reading the relevant memory:// files surfaced by those tools. Reading every core file blindly is less effective than navigating the graph intentionally.

## Edge Case Recovery Patterns

### Empty SearchMemories Results
→ Confirms knowledge doesn't exist → Proceed to WriteMemory with appropriate [[concepts]]

### No RecentActivity
→ Clean slate scenario → Start with SearchMemories to explore existing [[concepts]]

### BuildContext Reports "Concept not found"
→ Run Sync first → Retry BuildContext → If still missing, concept is truly new

### Sync Errors or Warnings
→ Check database permissions → Review recent file changes → Retry with clean state

### Zero Results from Multiple Tools
→ Validates creating new knowledge area → Use WriteMemory to establish foundation

## Tool Selection Decision Framework

The core project files still live under memory:// and remain the canonical written record. They also seed the knowledge graph, so they MUST stay accurate and [[concept]] rich.

flowchart TD
    PB[memory://projects/{project-name}/projectbrief.md] --> PC[memory://projects/{project-name}/productContext.md]
    PB --> SP[memory://projects/{project-name}/systemPatterns.md]
    PB --> TC[memory://projects/{project-name}/techContext.md]

    PC --> AC[memory://projects/{project-name}/activeContext.md]
    SP --> AC
    TC --> AC

    AC --> P[memory://projects/{project-name}/progress.md]

### Core Files (Required)
Standard structure under `memory://projects/{project-name}/`:

1. **projectbrief.md** - Foundation, scope, stakeholders, guardrails
2. **productContext.md** - Motivation, user journeys, [[product]] concepts
3. **activeContext.md** - Current priorities, next steps ⭐ Start here with BuildContext
4. **systemPatterns.md** - Architecture, integrations, invariants (use with Visualize)
5. **techContext.md** - Tooling, constraints, environment notes
6. **progress.md** - Status log with links to SequentialThinking sessions and assumptions

For detailed schemas and content requirements, see the memory:// files themselves.

### Additional Context
Create more files when they strengthen the graph:
- Deep dives that deserve their own [[concepts]]
- API and integration specs linked from activeContext
- Test strategies tied to [[quality]] or [[reliability]] concepts
- Deployment notes linked to [[operations]] topics

After writing or updating content, run `Sync` so these files become first-class citizens in search, context building, and visualization.

## Graph-First Reset Protocol

```text
1. Sync → RecentActivity → SearchMemories → BuildContext/Visualize
2. Read surfaced memory:// files (esp. core project docs)
3. Resume or spawn SequentialThinking sessions as needed
4. Log/verify assumptions in the Assumption Ledger
```

### Verification Checkpoints
After completing reset protocol, verify:
- ✓ Sync completed without errors (check concept count or graph structure)
- ✓ RecentActivity shows relevant activity OR confirms clean slate
- ✓ SearchMemories found relevant knowledge OR confirmed novelty
- ✓ BuildContext reveals relationships OR validates isolated concept

- **RecentActivity** identifies active sessions and documents you should revisit.
- **SearchMemories** (Hybrid) finds both textual and semantic matches for the current task.
- **BuildContext** reveals nearby concepts; **Visualize** converts them into diagrams to accelerate comprehension.
- **FindSimilarConcepts** highlights potential duplicates or adjacent topics before editing or consolidation.

## Core Workflows (Maenifold-Aware)

### Plan Mode
flowchart TD
    Start[Start] --> Sync[Run Sync (if needed)]
    Sync --> Activity[Inspect RecentActivity]
    Activity --> Discover[SearchMemories / FindSimilarConcepts]
    Discover --> Context[BuildContext / Visualize]
    Context --> Files[Read surfaced memory://]
    Files --> Plan[Create Plan or SequentialThinking Session]
    Plan --> Document[Document Plan in Chat + Reference URIs]

### Act Mode
flowchart TD
    Start[Start] --> Resume[Resume SequentialThinking or start new session]
    Resume --> Ledger[Append Assumptions if risks detected]
    Resume --> Execute[Implement / Investigate]
    Execute --> Capture[WriteMemory or EditMemory]
    Capture --> Sync[Run Sync to update graph]
    Sync --> Update[Update activeContext/progress]
    Update --> Document[Summarize results + references]

SequentialThinking sessions should cite relevant [[concepts]] and reference memory:// URIs. When finishing a session, write a conclusion with links to new or updated knowledge files.

## Documentation & Knowledge Updates

Update the knowledge base when:
1. Discovering new patterns or decisions (SequentialThinking conclusion → WriteMemory/EditMemory → Sync)
2. Completing significant implementation work (document in progress.md + related files)
3. Validating or refuting assumptions (update Assumption Ledger status, then Sync)
4. Responding to **update memory://** (review ALL core files, run SearchMemories to find related artifacts, ensure activeContext/progress reflect reality)

flowchart TD
    Start[Update Trigger]

    subgraph Process
        P1[Search existing knowledge]
        P2[Read & edit memory:// files]
        P3[SequentialThinking conclusion / assumptions]
        P4[WriteMemory or EditMemory to capture changes]
        P5[Run Sync + Validate via BuildContext]
    end

    Start --> Process

## Concept Tagging Rules

- ALWAYS include [[concepts]] when writing or editing to keep the graph connected.
- ALWAYS use [[WikiLink]] format to prevent graph corruption

### Core Format
- Double brackets: [[concept]] never [concept]
- Normalized to lowercase-with-hyphens internally

### Anti-Corruption Rules
- SINGULAR for general: [[tool]], [[agent]], [[test]]
- PLURAL only for collections: [[tools]] when meaning "all tools"
- PRIMARY concept only: [[MCP]] not [[MCP-server]]
- GENERAL terms: [[authentication]] not [[auth-system]]
- NO file paths: [[configuration]] not [[/path/to/config.json]]
- NO code elements: [[authentication]] not [[authService.GetToken()]]
- NO trivial words: Don't tag [[the]], [[a]], [[concept]], [[file]], [[system]] unless they're the actual topic
- TAG substance: [[machine-learning]], [[GraphRAG]], [[vector-embeddings]] - meaningful domain concepts
- REUSE existing [[concept]] before inventing near-duplicate (guard [[fragmentation]])
- HYPHENATE multiword: [[null-reference-exception]] not [[Null Reference Exception]]


### Examples
CORRECT: Fixed [[null-reference-exception]] in [[authentication]] using [[JWT]]
WRONG: Fixed [[NullReferenceException]] in [[auth-system]] using [[jwt-tokens]]

WHY: Concepts normalize to lowercase-with-hyphens internally. Use that format directly for consistency.


## Key reminders:
- Use get_help the first time you encounter a tool to understand its purpose and parameters
- After errors or when parameter requirements are unclear, consult get_help for troubleshooting
- After WriteMemory/EditMemory/RepairConcepts, run `Sync` so SearchMemories, BuildContext, and Visualize stay accurate.
- Use RepairConcepts (dry run first) only after AnalyzeConceptCorruption + FindSimilarConcepts confirm the merge is semantically safe.
- Reference assumption URIs in activeContext/progress so RecentActivity and the graph expose decision history.

By treating memory:// as graph seeds and routinely leveraging Maenifold tools, every reset becomes a quick rehydration from a living knowledge system rather than a manual file audit.
