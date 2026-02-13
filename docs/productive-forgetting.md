# Productive Forgetting

> What an AI learns to ignore is as important as what it remembers.

---

## The Problem Everyone Else Solves Wrong

Every AI memory system competes on retention: "Never lose context." "Remember everything." "Persistent memory." They share a premise: *more memory = better*.

This premise is wrong. The research is unambiguous:

> "The goal of memory is NOT information transmission through time. The goal is to optimize decision-making." — Richards & Frankland (2017), *Neuron*

A system that remembers everything but surfaces irrelevant memories at decision time is *worse* than a system that remembers less but surfaces the right things. This is context rot — old thoughts competing with new conclusions for the attention budget.

## What Productive Forgetting Is

Productive forgetting is the principle that **controlled deprioritization of stale memory improves system performance**. It has three components:

### 1. Decay: Freshness as Signal

Memories lose ranking weight over time via power-law decay (calibrated to ACT-R's empirically-validated d=0.5 parameter). A memory written 90 days ago with no access since doesn't disappear — it ranks lower in search and context injection. The information survives. The priority doesn't.

```
decay_weight = 1.0                              (within grace period)
decay_weight = 2^(-(days - grace) / half_life)   (after grace period)
```

| Memory Type | Grace Period | Half-Life | Rationale |
|---|---|---|---|
| Episodic (thinking sessions) | 7 days | 30 days | Task-bound, context-specific |
| Semantic (knowledge files) | 14 days | 30 days | General knowledge |
| Procedural (workflows) | 14 days | 30 days | Multi-step processes |

### 2. Access Boosting: Use Keeps Things Alive

Only *deliberate* access resets decay. Reading a memory (ReadMemory) resets its decay clock. Appearing in search results (SearchMemories) does not. Being traversed by BuildContext does not.

This prevents "accidental immortality" — where memories survive indefinitely because they happen to co-occur with active queries, not because anyone actually needs them.

### 3. Epistemic Pressure: Unexamined Beliefs Fade

The assumption ledger applies decay by validation status:

| Status | Decay Behavior |
|---|---|
| Validated | No decay — confirmed knowledge persists |
| Active | Normal decay — pressure to validate |
| Refined | Normal decay — superseded; should fade |
| Invalidated | Aggressive decay — historical record only |

If you never check whether something is true, the system gradually stops surfacing it. Socrates as infrastructure.

## How It's Different from Deletion

Decayed memories are not deleted. They're deprioritized.

A memory with decay weight 0.01 still:
- Exists at its `memory://` URI
- Can be retrieved by direct access (ReadMemory)
- Appears in targeted search (with low ranking)
- Maintains all `[[WikiLink]]` connections in the graph

It just doesn't win contests for your attention budget. This is how biological memory works: you haven't forgotten your childhood phone number — the access path decayed. If someone shows it to you, you recognize it.

## The Biological Basis

Productive forgetting is grounded in neuroscience, not heuristics:

- **Ebbinghaus (1885)**: Forgetting follows a power-law curve — rapid early forgetting, slow late persistence
- **ACT-R (Anderson, 1998)**: Base-level activation uses t^(-0.5) decay, validated across hundreds of experiments
- **Synaptic Homeostasis Hypothesis (Tononi & Cirelli, 2014)**: Sleep exists partly to prune weak synapses — wake strengthens connections, sleep renormalizes them
- **Richards & Frankland (2017)**: Memory optimizes *decision-making*, not *information retention* — forgetting prevents overfitting to noise

The Cognitive Sleep Cycle implements this in maenifold: periodic consolidation (episodic → semantic), decay processing, concept repair, and dream synthesis for novel connections.

Full treatment: [`docs/research/decay-in-ai-memory-systems.md`](research/decay-in-ai-memory-systems.md) (29 citations)

## Why This Matters for AI Agents

Long-running agents accumulate context across sessions. Without forgetting:
- Search results fill with stale debugging notes from months ago
- Context injection surfaces irrelevant memories, consuming attention budget
- Every new session competes with every old session for relevance
- The agent gets *worse* over time, not better

With productive forgetting:
- Recent, frequently-used knowledge naturally surfaces first
- Stale context fades without manual cleanup
- The knowledge graph stays *opinionated* — shaped by actual use patterns
- The agent develops something like accumulated preferences about what matters

## The Evidence

The central claim — decay improves retrieval — is measured by five benchmarks in `tests/Maenifold.Tests/DecayBenchmarkTests.cs`:

| Benchmark | Question | Result |
|---|---|---|
| Context Rot | Does recent signal outrank accumulated noise? | Decision doc ranks #1 out of 6 (fused score 0.033 vs 0.007) |
| Weight Distribution | Does the decay curve produce meaningful separation? | 18.4x suppression ratio between fresh and year-old content |
| Access Boosting | Does reading rescue actively-used old content? | Weight jumps from 0.18 to 1.0 after ReadMemory |
| Tiered Decay | Does episodic memory fade faster than semantic? | Episodic = 0.58 vs semantic = 1.0 at day 10 |
| Precision@3 | Does the right answer land in the top 3? | #1 out of 8 results, above 3 tangentially-related old files |

The weight distribution across the decay curve (power-law, d=0.5, 14-day grace):

```
Day  1:  1.000  (grace period — full weight)
Day 14:  1.000  (grace boundary — still full)
Day 30:  0.707  (moderate — clearly present)
Day 60:  0.177  (significant — deprioritized)
Day 90:  0.127  (heavy — nearly invisible in ranking)
Day 365: 0.054  (extreme — effectively buried unless directly accessed)
```

## The Competitive Position

No other AI memory system has built forgetting into its *identity*:

| System | Forgetting Approach |
|---|---|
| MemGPT | Virtual context management, no explicit decay |
| Zep | Recency scoring, but no biological grounding or sleep cycle |
| Generative Agents | Exponential recency decay on memory streams |
| **maenifold** | Power-law decay, tiered half-lives, access boosting, cognitive sleep cycle, epistemic pressure |

maenifold is the only system where forgetting is documented, researched, benchmarked, and biologically grounded.

---

*See also: [EVOLUTION.md](EVOLUTION.md) for the intellectual history of this concept.*
*See also: [decay-in-ai-memory-systems.md](research/decay-in-ai-memory-systems.md) for the full research paper.*
