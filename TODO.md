# TODO

## T-SLEEP-MULTI-001: Multi-agent sleep cycle architecture

Refactor the Cognitive Sleep Cycle from single-agent sequential workflow to multi-agent parallel orchestration.

**RTM**: FR-7.10, FR-7.11, NFR-7.10.1-3, NFR-7.11.1-4

### Tasks

| T-ID | Task | RTM | Status |
|------|------|-----|--------|
| T-SLEEP-MULTI-001.1 | Refactor `sleep-cycle.json` as orchestrator (dispatch + review + wake prep) | T-SLEEP-MULTI-001.1 | Planned |
| T-SLEEP-MULTI-001.2 | Create `sleep-consolidation.json` (replay → consolidation → dream synthesis) | T-SLEEP-MULTI-001.2, T-SLEEP-SAFETY-001.1 | Planned |
| T-SLEEP-MULTI-001.3 | Create `sleep-decay.json` (replay → decay processing, `list_memories` only) | T-SLEEP-MULTI-001.3, T-SLEEP-SAFETY-001.2 | Planned |
| T-SLEEP-MULTI-001.4 | Create `sleep-repair.json` (replay → concept repair, graph tools only) | T-SLEEP-MULTI-001.4, T-SLEEP-SAFETY-001.3 | Planned |
| T-SLEEP-MULTI-001.5 | Create `sleep-epistemic.json` (replay → assumption review, ledger only) | T-SLEEP-MULTI-001.5, T-SLEEP-SAFETY-001.4 | Planned |
| T-SLEEP-MULTI-001.6 | Manual test: run each specialist workflow independently | T-SLEEP-MULTI-001.6 | Planned |
| T-SLEEP-MULTI-001.7 | Manual test: run full orchestrated sleep cycle | T-SLEEP-MULTI-001.1 | Planned |

### Workflow Structure

```
sleep-cycle.json (Orchestrator - 3 steps)
├── Step 1: Dispatch 4 agents in parallel
│   ├── Agent runs: sleep-consolidation.json
│   ├── Agent runs: sleep-decay.json
│   ├── Agent runs: sleep-repair.json
│   └── Agent runs: sleep-epistemic.json
├── Step 2: Review all outputs
└── Step 3: Wake Preparation

sleep-consolidation.json (3 steps)
├── Step 1: Memory Replay (focus: consolidation candidates)
├── Step 2: Consolidation (episodic → semantic)
└── Step 3: Dream Synthesis (novel connections)

sleep-decay.json (2 steps)
├── Step 1: Memory Replay (focus: access patterns)
└── Step 2: Decay Processing (list_memories only)

sleep-repair.json (2 steps)
├── Step 1: Memory Replay (focus: concept usage)
└── Step 2: Concept Repair (graph tools only)

sleep-epistemic.json (2 steps)
├── Step 1: Memory Replay (focus: assumptions touched)
└── Step 2: Assumption Review (ledger only)
```

### Tool Access Safety Rules

| Workflow | ✅ Allowed | ❌ Forbidden | Rationale |
|----------|-----------|--------------|-----------|
| Consolidation | `read_memory`, `write_memory`, `edit_memory` | — | Access boosting is correct |
| Decay | `list_memories`, `recent_activity`, `write_memory` | `read_memory` | Preserve decay state |
| Repair | `analyze_concept_corruption`, `repair_concepts`, `sync`, `write_memory` | `read_memory` | Graph ops only |
| Epistemic | `assumption_ledger`, `search_memories`, `write_memory` | `read_memory` | Ledger is separate |
