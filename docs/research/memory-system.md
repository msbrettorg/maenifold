# Memory System: Theoretical Foundations

Maenifold's memory system implements principles from cognitive psychology and neuroscience. This document maps each feature to its canonical academic source.

## Overview

| Feature | Source | Application |
|---------|--------|-------------|
| **Two-Stage Model** | Rasch & Born, 2013 | `memory://thinking/` (episodic) vs `memory://research/` (semantic) |
| **ACT-R Decay** | Anderson & Lebiere, 1998 | Power-law decay with d=0.5 parameter |
| **Storage vs Retrieval** | Bjork & Bjork, 1992 | WikiLinks persist; decay affects ranking only |
| **Maintenance Cycles** | Tononi & Cirelli, 2014 | 4 parallel workflows during "sleep" |
| **Consolidation** | Diekelmann & Born, 2010 | Episodic → semantic transfer |

---

## Two-Stage Model

**Concept:** Fast episodic encoding → slow semantic consolidation

**Canonical Source:**
> Rasch, B., & Born, J. (2013). About sleep's role in memory. *Physiological Reviews*, 93(2), 681-766.
>
> DOI: [10.1152/physrev.00032.2012](https://doi.org/10.1152/physrev.00032.2012)

**Key Insight:**

The brain uses a two-stage memory system:
1. **Stage 1 (Hippocampus):** Rapid encoding, temporary storage—captures episodes as they occur
2. **Stage 2 (Neocortex):** Slow learning, permanent storage—extracts statistical regularities over time

The hippocampus acts as an "internal trainer" of the neocortex during sleep. This is not passive protection but active transformation—memories are restructured, generalized, and integrated with existing knowledge.

**Maenifold Implementation:**
- `memory://thinking/` — Episodic tier with 7-day grace period (fast decay)
- `memory://research/` — Semantic tier with 14-day grace period (slow decay)
- `memory-consolidation` workflow — Transfers high-value episodic content to semantic memory

---

## ACT-R Decay

**Concept:** Memories fade without access following power-law

**Canonical Source:**
> Anderson, J.R., & Lebiere, C. (1998). *The Atomic Components of Thought*. Lawrence Erlbaum Associates.
>
> DOI: [10.4324/9781315805696](https://doi.org/10.4324/9781315805696)

**Key Insight:**

ACT-R's base-level activation equation:

```
B_i = ln(Σ_{j=1}^{n} t_j^{-d})
```

Where:
- **B_i** = base-level activation of memory chunk *i*
- **n** = number of accesses (frequency effect)
- **t_j** = time since *j*-th access (recency effect)
- **d** = decay parameter (empirically **d = 0.5**)

When d = 0.5:
- Memory contribution halves when time **quadruples** (not doubles)
- This produces rapid early forgetting, slow late forgetting
- Very old memories can persist indefinitely

**Maenifold Implementation:**
- `DecayCalculator.cs` — Calculates retrieval strength using power-law formula
- Default d=0.5 produces "memory halves when time quadruples" behavior
- Access boosting: `ReadMemory` resets decay clock (adds new term to activation sum)

---

## Storage vs Retrieval Strength

**Concept:** Pointers persist; only accessibility fades

**Canonical Source:**
> Bjork, R.A., & Bjork, E.L. (1992). A new theory of disuse and an old theory of stimulus fluctuation. In A.F. Healy, S.M. Kosslyn, & R.M. Shiffrin (Eds.), *From Learning Processes to Cognitive Processes: Essays in Honor of William K. Estes* (Vol. 2, pp. 35-67). Erlbaum.
>
> Link: [ResearchGate](https://www.researchgate.net/publication/281322665_A_new_theory_of_disuse_and_an_old_theory_of_stimulus_fluctuation)

**Key Insight:**

Memory has two independent strengths:
1. **Storage strength:** How well-learned something is (stable, only increases)
2. **Retrieval strength:** How accessible it currently is (variable, decays without use)

Forgetting reflects temporary retrieval failure, not permanent storage loss. The information is "in there" but inaccessible. Critically, **retrieval practice restores access**—each successful retrieval strengthens the memory trace.

**Maenifold Implementation:**
- **WikiLinks** = storage strength (pointers persist, never deleted)
- **Decay weight** = retrieval strength (access recency determines surfacing priority)
- Files are never deleted; decay affects search ranking only
- `ReadMemory` restores retrieval strength (access boosting)

---

## Maintenance Cycles

**Concept:** Periodic graph hygiene (consolidation/pruning during sleep)

**Canonical Source:**
> Tononi, G., & Cirelli, C. (2014). Sleep and the price of plasticity: From synaptic and cellular homeostasis to memory consolidation and integration. *Neuron*, 81(1), 12-34.
>
> DOI: [10.1016/j.neuron.2013.12.025](https://doi.org/10.1016/j.neuron.2013.12.025)
>
> PMC: [PMC3921176](https://pmc.ncbi.nlm.nih.gov/articles/PMC3921176/)

**Key Insight:**

The Synaptic Homeostasis Hypothesis (SHY):

> "Sleep is the price the brain pays for plasticity."

Core claims:
1. **Wake = net synaptic potentiation:** Learning requires strengthening connections
2. **Sleep = net synaptic depression:** Renormalization restores homeostasis
3. **Down-selection:** Strongly-activated synapses survive; weak ones are pruned

This explains why sleep deprivation impairs learning—without renormalization, the system saturates.

**Maenifold Implementation:**

Four parallel maintenance workflows:
- `memory-consolidation` — Distills episodic → semantic (slow-wave sleep analog)
- `memory-decay` — Applies tiered decay weights (synaptic downscaling)
- `memory-repair` — Normalizes WikiLink variants (graph hygiene)
- `memory-epistemic` — Reviews assumptions, validates/invalidates (belief maintenance)

Triggered via `memory-cycle` workflow (agent-initiated or scheduled).

---

## Consolidation

**Concept:** CoT → long-term memory promotion + cross-domain linking

**Canonical Source:**
> Diekelmann, S., & Born, J. (2010). The memory function of sleep. *Nature Reviews Neuroscience*, 11(2), 114-126.
>
> DOI: [10.1038/nrn2762](https://doi.org/10.1038/nrn2762)

**Key Insight:**

Sleep-dependent consolidation is not mere repetition—it promotes both quantitative and qualitative changes:

1. **Hippocampal replay:** Sharp-wave ripples (100-300 Hz) reactivate memories
2. **Spindle-ripple coupling:** Precise timing transfers memories to neocortex
3. **Schema integration:** New memories connect with existing knowledge structures
4. **Gist extraction:** Details fade; statistical regularities strengthen

The process extracts what's important and integrates it with prior knowledge.

**Maenifold Implementation:**
- `SequentialThinking` sessions capture episodic reasoning traces
- `memory-consolidation` workflow identifies high-value sessions
- `FindSimilarConcepts` discovers cross-domain connections (semantic neighbors)
- Consolidated content moves from `memory://thinking/` to `memory://research/`

---

## Design Principles

These foundations inform maenifold's memory architecture:

1. **Soft decay, never delete:** Retrieval strength fades; storage strength persists
2. **Access boosting:** Deliberate retrieval restores accessibility
3. **Tiered half-lives:** Episodic (fast) vs semantic (slow) decay rates
4. **Periodic maintenance:** Consolidation requires active "sleep" cycles
5. **Graph as external memory:** WikiLinks are pointers; content retrieves on demand

The goal of memory is not information transmission through time—it's optimization of decision-making (Richards & Frankland, 2017).

---

## References

1. Anderson, J.R., & Lebiere, C. (1998). *The Atomic Components of Thought*. Lawrence Erlbaum. [DOI](https://doi.org/10.4324/9781315805696)

2. Bjork, R.A., & Bjork, E.L. (1992). A new theory of disuse and an old theory of stimulus fluctuation. In Healy et al. (Eds.), *From Learning Processes to Cognitive Processes* (Vol. 2, pp. 35-67). [ResearchGate](https://www.researchgate.net/publication/281322665_A_new_theory_of_disuse_and_an_old_theory_of_stimulus_fluctuation)

3. Diekelmann, S., & Born, J. (2010). The memory function of sleep. *Nature Reviews Neuroscience*, 11(2), 114-126. [DOI](https://doi.org/10.1038/nrn2762)

4. Rasch, B., & Born, J. (2013). About sleep's role in memory. *Physiological Reviews*, 93(2), 681-766. [DOI](https://doi.org/10.1152/physrev.00032.2012)

5. Richards, B.A., & Frankland, P.W. (2017). The persistence and transience of memory. *Neuron*, 94(6), 1071-1084. [DOI](https://doi.org/10.1016/j.neuron.2017.04.037)

6. Tononi, G., & Cirelli, C. (2014). Sleep and the price of plasticity. *Neuron*, 81(1), 12-34. [DOI](https://doi.org/10.1016/j.neuron.2013.12.025)
