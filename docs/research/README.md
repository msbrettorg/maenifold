# Research Index: Design Rationale

This folder captures the **why** behind maenifold’s architecture choices.  
Read these as design rationale for implementation decisions, not as publication artifacts.

## Paper index

| Paper | One-line focus | Key concepts |
|---|---|---|
| [`context-engineering.md`](./context-engineering.md) | Maps Anthropic-style context engineering principles directly onto maenifold primitives and workflows. | context rot, just-in-time retrieval, attention budget, progressive disclosure, subagent compaction |
| [`productive-forgetting.md`](./productive-forgetting.md) | Argues that selective forgetting improves decision quality more than brute-force retention. | decay curves, freshness weighting, access boosting, retrieval quality, context hygiene |
| [`memory-system.md`](./memory-system.md) | Connects maenifold memory tiers and maintenance workflows to cognitive science foundations. | episodic vs semantic memory, ACT-R decay, consolidation, sleep-cycle maintenance |
| [`symbolic-communication-in-ai-systems.md`](./symbolic-communication-in-ai-systems.md) | Positions WikiLink pointers as a practical communication protocol for multi-agent systems. | symbolic pointers, concept-as-protocol, multi-agent communication, interpretability, graph resolution |
| [`confession-reports.md`](./confession-reports.md) | Defines inference-time honesty enforcement via structured self-reporting and adversarial checks. | ConfessionReport, policy compliance, red-team validation, evidence trails, accountability |
| [`decay-in-ai-memory-systems.md`](./decay-in-ai-memory-systems.md) | Broad survey of forgetting mechanisms and how they inform long-lived agent memory maintenance. | forgetting curves, power-law decay, cognitive sleep cycle, consolidation/decay/synthesis, ranking-not-deletion |

## Recommended reading order (newcomers)

1. **[`context-engineering.md`](./context-engineering.md)** — Start with the top-level problem: finite context and token signal quality.
2. **[`productive-forgetting.md`](./productive-forgetting.md)** — Internalize the core principle: ranking relevance beats remembering everything.
3. **[`memory-system.md`](./memory-system.md)** — See how the principle becomes concrete memory tiers and maintenance workflows.
4. **[`symbolic-communication-in-ai-systems.md`](./symbolic-communication-in-ai-systems.md)** — Understand why `[[WikiLinks]]` are treated as protocol-level infrastructure.
5. **[`confession-reports.md`](./confession-reports.md)** — Review governance and honesty-enforcement patterns used in delivery workflows.
6. **[`decay-in-ai-memory-systems.md`](./decay-in-ai-memory-systems.md)** — Finish with the deep survey for full theoretical grounding and broader references.

## How to use this folder

- Use these docs when making architecture/product decisions about memory, retrieval, workflows, and agent coordination.
- Prefer extracting implementation implications into code/docs close to the subsystem being changed.
- Keep additions scoped to decisions that materially affect maenifold’s design.
