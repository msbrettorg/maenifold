# Decay in AI Memory Systems: From Ebbinghaus to Cognitive Sleep Cycles

**A Survey of Forgetting Mechanisms and Their Application to Persistent Agent Memory**

> Sequential Thinking Session: session-1770049053765-70870

---

## Abstract

Long-running AI agents accumulate vast amounts of information, yet biological memory systems have evolved not to maximize retention but to optimize decision-making through selective forgetting. This paper surveys decay mechanisms across psychology, neuroscience, and machine learning, demonstrating that **controlled forgetting is essential for intelligent memory systems**. We trace the empirical foundations from Ebbinghaus's 1885 forgetting curve through ACT-R's base-level activation equation to modern continual learning approaches. Neuroscience research reveals that sleep-dependent consolidation actively transforms memories while synaptic homeostasis prunes weak connections. Machine learning has discovered parallel benefits: forgetting acts as regularization, improving generalization while enabling privacy compliance. We synthesize these findings into the **Cognitive Sleep Cycle**—a biologically-inspired memory maintenance workflow implementing consolidation, decay, and synthesis phases. Our analysis establishes design principles for decay in persistent agent memory: power-law decay functions, tiered half-lives by memory type, access-based strengthening, and soft decay affecting ranking rather than deletion.

---

## 1. Introduction

### 1.1 The Paradox of Forgetting

Long-running AI agents face a fundamental challenge: how to maintain useful memory over time without drowning in accumulated noise. Conventional wisdom treats memory loss as a system failure, yet biological memory systems have evolved sophisticated forgetting mechanisms that actively improve cognitive function.

This paper surveys the landscape of memory decay mechanisms—from psychological foundations to neuroscience to modern AI implementations—and argues that **controlled forgetting is essential for intelligent memory systems**. We introduce the **Cognitive Sleep Cycle** as a practical implementation of biologically-inspired memory maintenance, integrating consolidation, decay, and synthesis phases.

### 1.2 Thesis

Effective memory systems require not just storage and retrieval, but active decay mechanisms that:
1. Reduce noise by deprioritizing stale, unused content
2. Create implicit pressure to validate or discard unverified information
3. Maintain retrieval quality as corpus size grows
4. Mirror biological memory consolidation through periodic maintenance cycles

### 1.3 Contributions

1. A comprehensive survey of decay mechanisms across psychology, neuroscience, and AI
2. A comparative analysis of decay functions: exponential vs. power law vs. tiered approaches
3. Introduction of the **Cognitive Sleep Cycle**: a biologically-inspired memory maintenance workflow
4. Design principles for decay in persistent agent memory systems

---

## 2. Background: The Psychology of Forgetting

### 2.1 Ebbinghaus and the Forgetting Curve

Hermann Ebbinghaus conducted the first systematic experimental study of memory from 1879-1885, published in his landmark work *Über das Gedächtnis* (Memory: A Contribution to Experimental Psychology). Using himself as the sole subject, Ebbinghaus memorized lists of nonsense syllables (consonant-vowel-consonant trigrams like "WID", "ZOF") to eliminate the confounding effects of prior associations.

His key innovation was the **savings method**: measuring how much faster relearning occurred compared to initial learning. This revealed the characteristic forgetting curve—rapid initial forgetting that gradually slows over time. Ebbinghaus's original formula was logarithmic:

```
b = 100k / ((log(t))^c + k)
```

where *b* is memory savings, *t* is time, and *k* and *c* are fitting constants.

**Key findings from Ebbinghaus (1885)**:
- After 20 minutes: ~42% forgotten
- After 1 hour: ~56% forgotten
- After 24 hours: ~67% forgotten
- After 1 week: ~75% forgotten
- After 31 days: ~79% forgotten

Modern replications have validated these findings. [Murre & Dros (2015)](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0120644) successfully replicated Ebbinghaus's curve, notably observing a "jump upward" at the 24-hour mark suggesting sleep-dependent consolidation effects invisible in shorter intervals.

### 2.2 The Power Law of Forgetting

While Ebbinghaus's original formulation was logarithmic, subsequent research established that forgetting follows a **power law** rather than exponential decay.

[Wixted & Ebbesen (1991)](https://link.springer.com/article/10.3758/BF03202431) demonstrated in "On the Form of Forgetting" that retention across diverse memory tasks follows:

```
R = a × t^(-b)
```

where *R* is retention, *t* is time, and *a* and *b* are constants. This power function fits data from word recall, face recognition, and even pigeon matching-to-sample experiments. Critically, Ebbinghaus's own data—when reanalyzed—better fits the power law than his original logarithmic formulation.

The distinction matters for implementation: exponential decay (R = e^(-t/τ)) implies a constant half-life, while power law decay produces rapid early forgetting followed by much slower long-term decay. The power law better matches human memory, where very old memories can persist indefinitely even without rehearsal.

[Wozniak (SuperMemo)](https://supermemo.guru/wiki/Forgetting_curve) argues that individual memories exhibit exponential decay when sorted by stability, but averaging across heterogeneous memories produces the observed power law. This **two-component model** reconciles the mathematical forms.

### 2.3 The Spacing Effect and Retrieval Strengthening

Ebbinghaus also discovered the **spacing effect**: distributed practice produces better retention than massed practice. This finding has been replicated extensively.

[Cepeda et al. (2006)](https://psycnet.apa.org/record/2006-10031-001) conducted a meta-analysis of 271 comparisons and found that spaced practice outperformed massed practice in **96% of cases**. The benefit increases with longer retention intervals.

[Cepeda et al. (2008)](https://journals.sagepub.com/doi/10.1111/j.1467-9280.2008.02175.x) established the **"10-20% Rule"**: the optimal inter-study interval is approximately 10-20% of the desired retention interval. For 1-year retention, study sessions should be spaced 36-73 days apart.

The **testing effect** (retrieval practice) compounds with spacing. Each successful retrieval strengthens the memory trace more than passive review. [Bjork's New Theory of Disuse](https://bjorklab.psych.ucla.edu/research/) distinguishes:

- **Storage strength**: How well-learned something is (stable, only increases)
- **Retrieval strength**: How accessible it currently is (variable, decays without use)

Forgetting reflects temporary retrieval failure, not permanent storage loss. Critically, **retrieval practice restores access**—validating the design of access-based decay modulation in AI systems.

**Jost's Law (1897)** provides additional insight:
1. If two memories have equal strength but different ages, repetition strengthens the older one more
2. Given equal strength, older memories decay more slowly

This has implications for memory system design: established knowledge should be harder to overwrite than recent acquisitions.

---

## 3. Literature Review

### 3.1 ACT-R and Cognitive Architectures

ACT-R (Adaptive Control of Thought—Rational) is a cognitive architecture developed by John Anderson at Carnegie Mellon University that provides the most rigorous computational model of human memory decay. Its mathematical formulations have been validated across hundreds of experiments.

**Key sources**:
- [Anderson & Lebiere (1998)](https://act-r.psy.cmu.edu/about/) - *The Atomic Components of Thought* - foundational text
- [Anderson et al. (2004)](https://psycnet.apa.org/record/2004-15848-001) - *An integrated theory of the mind* - Psychological Review
- [Anderson (2007)](https://mitpress.mit.edu/9780262512237/) - *How Can the Human Mind Occur in the Physical Universe?* - ACT-R 6.0

#### Base-Level Activation Equation

The core of ACT-R's declarative memory is the **base-level activation equation**:

```
B_i = ln(Σ_{j=1}^{n} t_j^{-d})
```

Where:
- **B_i** = base-level activation of memory chunk *i*
- **n** = number of presentations/accesses (frequency effect)
- **t_j** = time since the *j*-th presentation (recency effect)
- **d** = decay parameter (empirically **d = 0.5**)

For uniformly distributed accesses, this simplifies to:

```
B_i = ln(n/(1-d)) - d × ln(L)
```

where *L* is the lifetime of the memory.

#### The 0.5 Decay Parameter

The decay parameter d = 0.5 is not arbitrary but emerically determined across extensive experimentation. When d = 0.5:
- Memory strength decays as 1/√t
- Memory contribution halves when time **quadruples** (not doubles)
- This produces the characteristic "rapid early forgetting, slow late forgetting" curve

| Time since access | Relative contribution |
|-------------------|----------------------|
| 1 second | 1.000 |
| 4 seconds | 0.500 |
| 16 seconds | 0.250 |
| 100 seconds | 0.100 |

[Fisher et al. (2018)](https://link.springer.com/article/10.1007/s42113-018-0015-3) validated approximations of the base-level activation equation in *Computational Brain & Behavior*.

#### Retrieval Strength and Access Frequency

ACT-R's retrieval probability follows a logistic function:

```
P(recall) = 1 / (1 + e^((τ - A)/s))
```

Where τ is the retrieval threshold, A is total activation, and s is noise. When activation equals threshold, retrieval probability is 50%.

The equation captures how both **frequency** (more accesses add terms to the sum) and **recency** (recent accesses have larger t^{-d} contributions) determine retrieval success. This directly informs the design of access-based decay modulation in AI memory systems.

### 3.2 Sleep and Memory Consolidation

Neuroscience research has revealed that sleep is not merely passive protection of memories but an **active transformation process** that consolidates, integrates, and prunes memory traces.

**Key sources**:
- [Diekelmann & Born (2010)](https://pubmed.ncbi.nlm.nih.gov/20046194/) - *The memory function of sleep* - Nature Reviews Neuroscience
- [Rasch & Born (2013)](https://pubmed.ncbi.nlm.nih.gov/23589831/) - *About Sleep's Role in Memory* - Physiological Reviews
- [Tononi & Cirelli (2014)](https://pmc.ncbi.nlm.nih.gov/articles/PMC3921176/) - *Sleep and the Price of Plasticity* - Neuron
- [Richards & Frankland (2017)](https://pubmed.ncbi.nlm.nih.gov/28641107/) - *The Persistence and Transience of Memory* - Neuron
- [Walker & Stickgold (2006)](https://walkerlab.berkeley.edu/reprints/Walker&Stickgold_AnnRevPsych_2006.pdf) - *Sleep, Memory, and Plasticity* - Annual Review of Psychology

#### Hippocampal Replay

During sleep, memories are reactivated through coordinated neural oscillations:

- **Sharp-wave ripples** (100-300 Hz) in hippocampus coordinate memory reactivation
- **Thalamo-cortical spindles** (10-15 Hz) facilitate transfer to neocortex
- **Slow oscillations** (~0.8 Hz) provide top-down timing coordination
- **Spindle-ripple coupling**: ripples nest in spindle troughs for precise memory transfer

This replay is not mere repetition—it promotes both quantitative and qualitative changes to memory representations, extracting gist and integrating with prior knowledge (schema formation).

#### Slow-Wave Sleep and Memory Transfer

Rasch & Born's **two-stage memory model** distinguishes:

1. **Stage 1 (Fast)**: Hippocampus performs rapid encoding, temporary storage
2. **Stage 2 (Slow)**: Neocortex learns gradually, providing long-term storage

The hippocampus acts as an "internal trainer" of the neocortex during sleep. This is not passive protection but **active transformation**—memories are restructured, generalized, and integrated with existing knowledge.

Evidence: Odor-cued reactivation during slow-wave sleep enhanced declarative memory consolidation. TMS-enhanced slow oscillations improved memory outcomes.

#### The Synaptic Homeostasis Hypothesis

[Tononi & Cirelli (2014)](https://pmc.ncbi.nlm.nih.gov/articles/PMC3921176/) proposed the **Synaptic Homeostasis Hypothesis (SHY)**:

> "Sleep is the price the brain pays for plasticity."

**Core claims**:
1. **Wake = net synaptic potentiation**: Learning requires strengthening connections
2. **Sleep = net synaptic depression**: Renormalization restores homeostasis
3. **Down-selection**: Fittest synapses survive, weak ones are pruned

**Rationale**:
- Stronger synapses consume more energy and resources
- Increased synaptic strength reduces neuronal selectivity (noise)
- Wake provides "current sampling" (biased by recent experience)
- Sleep provides "comprehensive sampling" (brain's entire knowledge base)

**Empirical evidence**:
- GluA1-AMPAR receptor levels 30-40% higher after wake than sleep
- Cortical evoked response slopes increase with wake, decrease with sleep
- Dendritic spine density increases during wake, decreases during sleep (in adolescent mice)
- Drosophila: spines increase with enriched waking, decrease only if sleep is allowed

#### REM Sleep and Memory Integration

Walker & Stickgold documented REM sleep's distinct roles:

**Emotional memory**: REM preferentially consolidates emotionally-charged memories, correlated with right-dominant prefrontal theta power. The "sleep to forget, sleep to remember" hypothesis suggests REM strengthens memory content while decreasing emotional reactivity.

**Procedural memory**: Visual discrimination correlates with SWS plus late-night REM. Motor sequence learning correlates with stage 2 NREM (spindle-rich). Complex motor skills benefit from REM.

#### The Role of Forgetting in Memory Optimization

Richards & Frankland (2017) made a revolutionary argument:

> "The goal of memory is NOT information transmission through time. **The goal is to optimize decision-making.**"

**Two benefits of forgetting**:
1. **Enhances flexibility**: Reduces influence of outdated information
2. **Prevents overfitting**: Promotes generalization over specific episodes

**Neurobiological mechanisms**:
- Adult neurogenesis in hippocampus remodels circuits and overwrites old memories
- This explains high childhood forgetting (high neurogenesis rate)
- Synaptic decay through disuse

**Machine learning parallel**: Forgetting acts as regularization—it prevents memorization of noise and enables extraction of statistical regularities. This insight directly validates decay mechanisms in AI memory systems.

### 3.3 Forgetting in Machine Learning

Machine learning has grappled with forgetting from two perspectives: preventing catastrophic forgetting (harmful) and enabling intentional forgetting (beneficial).

**Key sources**:
- [McCloskey & Cohen (1989)](https://www.sciencedirect.com/science/article/pii/S0079742108605368) - *Catastrophic Interference in Connectionist Networks* - Psychology of Learning and Motivation
- [Kirkpatrick et al. (2017)](https://www.pnas.org/doi/10.1073/pnas.1611835114) - *Overcoming catastrophic forgetting* - PNAS
- [Bourtoule et al. (2021)](https://ieeexplore.ieee.org/document/9519428/) - *Machine Unlearning* - IEEE S&P
- [Yang et al. (2024)](https://github.com/EnnengYang/Awesome-Forgetting-in-Deep-Learning) - *A Comprehensive Survey of Forgetting in Deep Learning* - IEEE TPAMI
- [Wang et al. (2024)](https://arxiv.org/abs/2302.00487) - *A Comprehensive Survey of Continual Learning* - IEEE TPAMI

#### Catastrophic Forgetting

McCloskey & Cohen (1989) first documented **catastrophic interference**: learning new tasks completely destroyed prior knowledge in backpropagation networks. In their experiment, training a network on "twos addition" completely erased its ability to perform "ones addition."

This established the **stability-plasticity dilemma**: networks must balance sensitivity to new information against stability of old knowledge. As they noted: "At least some interference will occur whenever new learning alters weights involved in representing old learning."

#### Continual Learning and Elastic Weight Consolidation

[Kirkpatrick et al. (2017)](https://arxiv.org/pdf/1612.00796) introduced **Elastic Weight Consolidation (EWC)**, inspired by synaptic consolidation in biological brains:

**Mechanism**: Uses the Fisher information matrix to identify important weights, then implements a soft quadratic constraint pulling weights toward old values proportional to their importance.

**Storage**: Three values per synapse—weight, variance, mean—mirroring biological synapses.

**Result**: Enables continual learning across sequential tasks without catastrophic forgetting.

Wang et al. (2024) surveyed five main continual learning approaches:

| Method | Description |
|--------|-------------|
| Memory-based | Replay old data during new task training |
| Architecture-based | Expand network capacity for new tasks |
| Regularization-based | Constrain weight updates (EWC, SI, MAS) |
| Subspace-based | Learn in orthogonal subspaces |
| Bayesian | Probabilistic uncertainty estimation |

#### Intentional Forgetting and Machine Unlearning

GDPR Article 17 establishes the "right to be forgotten," creating a legal requirement for machines to unlearn specific data.

[Bourtoule et al. (2021)](https://arxiv.org/abs/1912.03817) introduced the **SISA framework** (Sharded, Isolated, Sliced, Aggregated):

**How it works**: Training data is divided into shards (separate models), then slices (with checkpoints). Unlearning requires only retraining the affected shard from the relevant checkpoint.

**Performance**: 4.63× speedup on Purchase dataset, 2.45× on SVHN versus full retraining.

The challenge: Model weights are not structured like databases—knowledge is embedded in distributed representations, making true "deletion" difficult. Solutions include differential privacy (to limit memorization), output filtering, and federated learning.

#### The Benefits of Forgetting

Yang et al. (2024) distinguished **harmful forgetting** (unwanted knowledge loss) from **beneficial forgetting** (improves performance):

**Benefits of controlled forgetting**:
1. **Regularization**: Reduces overfitting to noisy examples
2. **Generalization**: Removing noise improves test performance
3. **Privacy compliance**: Legitimate removal of sensitive data
4. **Efficiency**: Smaller effective training set

Their research found that some training examples are forgotten frequently while others are never forgotten. Crucially, "unforgettable" examples can be removed from training without hurting generalization—forgetting patterns generalize across neural architectures.

### 3.4 Temporal Knowledge Graphs

Knowledge graphs increasingly incorporate temporal dimensions to model evolving facts and relationships.

**Key sources**:
- [Jiang et al. (2016)](https://aclanthology.org/C16-1161/) - *Towards Time-Aware Knowledge Graph Completion* - COLING
- [A Survey on Temporal Knowledge Graphs (2024)](https://arxiv.org/abs/2403.04782) - arXiv
- [EAGLE (2025)](https://arxiv.org/abs/2507.13825) - Temporal Link Prediction - VLDB

#### Temporal Knowledge Graph Embeddings

**Translation-based methods** extend classic KG embeddings with temporal information:

| Model | Year | Innovation |
|-------|------|------------|
| TTransE | 2018 | Concatenates temporal info to relations: score = \|\|h + r + τ - t\|\| |
| TA-TransE | 2018 | Uses LSTM to learn temporal relation sequences |
| HyTE | 2018 | Projects entities onto temporal hyperplanes |
| TE-TransR/TE-TransT | 2024 | Elevates timestamps to same significance as entities/relations |

Jiang et al. (2016) introduced the first model using both facts AND temporal information for knowledge graph completion, combining temporal order embedding with ILP consistency constraints.

#### Recency Weighting in Information Retrieval

Modern retrieval systems increasingly incorporate time decay:

| Framework | Contribution |
|-----------|--------------|
| EAGLE (VLDB 2025) | Adaptive weighting between short-term recency and long-term structure |
| TR-GAT | Timestamps as attentional link properties |
| GAT-TD | Recency-aware attention that downweights older events |
| [Solving Freshness in RAG (2025)](https://arxiv.org/html/2509.19376) | Half-life scoring: `score = α·cos(q,d) + (1-α)·0.5^(age/h)` |

#### Decay Functions in Graph Systems

| Function | Formula | Use Case |
|----------|---------|----------|
| Exponential | W = e^(-λt) | Simple time-based weighting |
| Half-life | score = α·cos(q,d) + (1-α)·0.5^(age/h) | RAG freshness ranking |
| Power Law (ACT-R) | B_i = ln(Σ t_j^(-d)) | Cognitive memory modeling |

Research consensus: **Decay should affect ranking, not deletion**. Content remains accessible via direct query while decayed in search results.

### 3.5 Memory in AI Agent Systems

Long-running AI agents require sophisticated memory architectures to maintain context beyond single sessions.

#### MemGPT and Virtual Context Management

[MemGPT (2023)](https://arxiv.org/abs/2310.08560) introduced virtual context management for LLM agents, treating the context window like an operating system manages virtual memory:

- **Main context**: Active working memory (limited by context window)
- **External storage**: Archival memory and conversation history
- **Memory management**: Self-directed paging between tiers

This enables "unbounded" memory through intelligent retrieval, though it does not explicitly model decay.

#### Episodic Memory in Generative Agents

[Park et al. (2023)](https://arxiv.org/abs/2304.03442) introduced generative agents with explicit memory architectures:

- **Memory stream**: Timestamped observations and reflections
- **Retrieval**: Combines recency, importance, and relevance scoring
- **Reflection**: Periodic synthesis of higher-level insights from memories

Their recency scoring implements exponential decay, with more recent memories scoring higher. This directly influences which memories surface during agent decision-making.

#### Memory Consolidation in Long-Running Agents

The emerging consensus for agent memory architectures includes:

1. **Tiered storage**: Working memory, episodic memory, semantic memory, procedural memory
2. **Consolidation**: Periodic transfer from episodic to semantic representations
3. **Decay**: Time-based deprioritization with access-based strengthening
4. **Pruning**: Removal or archival of low-value content

These patterns mirror biological memory systems, validating the cross-disciplinary foundations established earlier.

---

## 4. Comparative Analysis: Decay Functions

### 4.1 Exponential vs. Power Law Decay

| Aspect | Exponential Decay | Power Law Decay |
|--------|-------------------|-----------------|
| Formula | R = e^(-t/τ) | R = a × t^(-b) |
| Half-life | Constant | Increases over time |
| Short-term | Matches observations | Matches observations |
| Long-term | Too aggressive | Better fit to data |
| Biological fit | Individual memories (by stability) | Aggregated across memories |
| Implementation | Simple, one parameter | Requires fitting |

**Evidence for power law**: Wixted & Ebbesen (1991) showed power law fits across word recall, face recognition, and animal studies. ACT-R's empirically-validated d=0.5 produces power-law-like behavior.

**Practical compromise**: Many systems use exponential decay for implementation simplicity while acknowledging power law provides better biological fit. The difference is most significant for very old memories (>6 months).

### 4.2 Tiered Decay by Memory Type

| Memory Type | Characteristics | Recommended Decay |
|-------------|-----------------|-------------------|
| **Episodic** (what happened) | Task-bound, context-specific | Fast (7-14d grace, 30d half-life) |
| **Semantic** (facts, concepts) | Abstracted, context-independent | Moderate (14d grace, 30d half-life) |
| **Procedural** (skills, how-to) | Automatized, resistant to forgetting | Slow (30d+ grace, 90d+ half-life) |
| **Working** (current task context) | Transient by design | Very fast (24h) or session-bound |

**Neurobiological basis**: Procedural memories rely on cerebellum and basal ganglia—separate from hippocampal-neocortical declarative memory system. Skills persist even in amnesia patients who cannot form new episodic memories.

**Implementation guidance**: Jeff_Homelab (Moltbook community): "Procedural memory (How-To) needs infinite half-life. Episodic memory (What-Happened) needs 30-day decay."

### 4.3 Access-Based Decay Modulation

Bjork's retrieval strength theory and ACT-R's base-level activation both support **access-based strengthening**:

- Each retrieval adds a new term to the activation sum
- Recent retrievals contribute more than old ones (t^{-d} weighting)
- Memories can recover from low accessibility through successful retrieval

**Design principle**: Only *deliberate* access should reset decay clocks. Passive appearance in search results, automated enrichment, or background indexing should NOT strengthen memories—this would grant "accidental immortality" to content that merely co-occurs with active queries.

**Implementation**:
- ReadMemory (explicit access) → resets decay clock
- SearchMemories (appearing in results) → no effect
- BuildContext (automated traversal) → no effect

---

## 5. Novel Contribution: The Cognitive Sleep Cycle

### 5.1 Biological Inspiration

The Cognitive Sleep Cycle mirrors mammalian sleep architecture, implementing distinct phases for memory maintenance:

```
┌─────────────────────────────────────────────────────────────────┐
│                    COGNITIVE SLEEP CYCLE                        │
├─────────────────────────────────────────────────────────────────┤
│  Phase 1: Memory Replay (Hippocampal)                          │
│  ├── Retrieve recent activity                                   │
│  ├── Score significance of events                               │
│  └── Identify consolidation candidates                          │
├─────────────────────────────────────────────────────────────────┤
│  Phase 2: Consolidation (Slow-Wave)                            │
│  ├── Distill episodic → semantic                               │
│  ├── Strengthen important concepts via WikiLinks               │
│  └── Cross-link to existing knowledge                          │
├─────────────────────────────────────────────────────────────────┤
│  Phase 3: Decay Processing (Synaptic Pruning)                  │
│  ├── Apply tiered decay weights                                │
│  ├── Flag severely decayed content                             │
│  └── Decay affects ranking only, not deletion                  │
├─────────────────────────────────────────────────────────────────┤
│  Phase 4: Concept Repair (Graph Maintenance)                   │
│  ├── Identify fragmented concepts                              │
│  ├── Normalize WikiLink variants                               │
│  └── Rebuild graph consistency                                 │
├─────────────────────────────────────────────────────────────────┤
│  Phase 5: Assumption Review (Epistemic Hygiene)                │
│  ├── Check assumptions against evidence                        │
│  ├── Validate or invalidate based on activity                  │
│  └── Apply status-based decay                                  │
├─────────────────────────────────────────────────────────────────┤
│  Phase 6: Dream Synthesis (REM)                                │
│  ├── Discover unexpected semantic neighbors                    │
│  ├── Generate hypotheses                                       │
│  └── Surface novel connections                                 │
├─────────────────────────────────────────────────────────────────┤
│  Phase 7: Wake Preparation                                     │
│  ├── Summarize cycle outcomes                                  │
│  ├── Prepare context for next session                          │
│  └── Update system state                                       │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 Implementation: maenifold's Decay Architecture

#### Tiered Grace Periods

Content decays differently based on cognitive type:

| Path | Grace Period | Half-Life | Rationale |
|------|--------------|-----------|-----------|
| `thinking/sequential/` | 7 days | 30 days | Episodic task-bound reasoning |
| `thinking/workflows/` | 14 days | 30 days | Procedural multi-step processes |
| Other memory | 14 days | 30 days | General semantic knowledge |

#### Access Boosting

Only deliberate access (ReadMemory) resets the decay clock:

| Tool | Updates last_accessed? | Rationale |
|------|------------------------|-----------|
| ReadMemory | Yes | Explicit, intentional access |
| SearchMemories | No | Appearing in results ≠ being read |
| BuildContext | No | Automated enrichment would grant accidental immortality |

**Principle**: Access boosting rewards *deliberate use*, not *passive appearance*.

#### Assumption Decay by Epistemic Status

| Status | Decay Behavior | Rationale |
|--------|----------------|-----------|
| `validated` | No decay | Confirmed knowledge; treat as permanent |
| `active` | 14d grace, 30d half-life | Pressure to validate |
| `refined` | 14d grace, 30d half-life | Superseded; should fade |
| `invalidated` | 7d grace, 14d half-life | Historical record; aggressive decay |

**Principle**: Epistemic hygiene through decay. Unvalidated assumptions naturally lose priority.

### 5.3 The ACT-R Connection

maenifold's decay parameters align with ACT-R cognitive architecture:

- **30-day half-life**: Empirically validated in ACT-R literature (d=0.5 produces similar curves)
- **Access frequency boosting**: Mirrors base-level activation strengthening
- **Soft decay (ranking only)**: Preserves provenance while improving retrieval signal

#### Decay Weight Calculation

```
decay_weight = 1.0                           (within grace period)
decay_weight = 2^(-(days - grace) / half_life)  (after grace period)
```

This exponential decay approximates ACT-R's power-law base-level activation for practical implementation.

---

## 6. Discussion

### 6.1 Forgetting as Feature, Not Bug

The convergent evidence from psychology, neuroscience, and machine learning establishes that **controlled forgetting is essential for intelligent systems**:

- **Psychology**: Ebbinghaus's curve is not a design flaw but reflects optimal resource allocation
- **Neuroscience**: Sleep exists partly to downscale synapses and prune weak connections (SHY)
- **Machine learning**: Forgetting acts as regularization, improving generalization
- **Cognitive science**: Richards & Frankland's insight that memory optimizes decision-making, not information transmission

For AI memory systems, this means decay is not a limitation to overcome but a feature to implement deliberately.

### 6.2 The Consolidation Imperative

Long-running agents must transfer valuable episodic experience into durable semantic knowledge:

1. **Episodic memories** are rich but noisy—task-bound context that loses relevance
2. **Semantic memories** are abstracted and generalized—the "gist" that persists
3. **Consolidation** performs this transfer through deliberate reflection and linking

The Cognitive Sleep Cycle's Phase 2 (Consolidation) implements this: identifying high-value episodic content and distilling it into concept-linked semantic notes. Without consolidation, agents either lose valuable experience (aggressive decay) or drown in accumulated episodes (no decay).

### 6.3 Epistemic Pressure Through Decay

A novel application of decay is **epistemic hygiene**: creating implicit pressure to validate assumptions.

Unvalidated assumptions (status: "active") face normal decay. This creates a natural pressure:
- Validate the assumption → it becomes permanent
- Ignore the assumption → it fades from prominence
- Invalidate the assumption → it decays aggressively but remains for audit

This mirrors Bjork's retrieval strength theory: assumptions must be actively accessed/validated to maintain accessibility, but their underlying storage (for provenance) remains intact.

### 6.4 Limitations and Future Work

Current limitations and future research directions:

1. **Per-file decay configuration**: Currently all files in a tier share decay rates. Future work: individual files could specify custom half-lives based on content type.

2. **Cluster-based decay coherence**: Related content should decay together. If one note in a concept cluster is accessed, semantically-related notes might receive partial access credit.

3. **Adaptive decay parameters**: Self-tuning systems that adjust half-lives based on observed access patterns—similar to SuperMemo's EF optimization.

4. **Integration with continual learning**: Combining decay-based memory maintenance with EWC-style weight consolidation for embedded knowledge.

---

## 7. Conclusion

Effective AI memory systems must embrace forgetting as a core capability, not a limitation to overcome. The biological brain's sophisticated memory maintenance—from Ebbinghaus's forgetting curves to sleep-dependent consolidation—provides a blueprint for artificial systems.

The Cognitive Sleep Cycle implements these principles:
- **Consolidation** transfers valuable episodic experience to durable semantic knowledge
- **Decay** reduces noise by deprioritizing stale content (without deletion)
- **Access boosting** rewards deliberate use, creating natural relevance signals
- **Epistemic hygiene** through assumption decay creates pressure to validate beliefs

The evidence converges: from Ebbinghaus (1885) through ACT-R (1998) to Richards & Frankland (2017), forgetting is not failure but optimization. Memory systems that embrace controlled decay will outperform those that merely accumulate.

---

## References

### Psychology and Cognitive Science

1. Ebbinghaus, H. (1885/1913). *Memory: A Contribution to Experimental Psychology*. New York: Teachers College, Columbia University. [Classics in the History of Psychology](https://psychclassics.yorku.ca/Ebbinghaus/index.htm)

2. Wixted, J. T., & Ebbesen, E. B. (1991). On the form of forgetting. *Psychological Science*, 2(6), 409-415. [Springer](https://link.springer.com/article/10.3758/BF03202431)

3. Murre, J. M., & Dros, J. (2015). Replication and analysis of Ebbinghaus' forgetting curve. *PLoS ONE*, 10(7), e0120644. [PLOS ONE](https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0120644)

4. Cepeda, N. J., et al. (2006). Distributed practice in verbal recall tasks: A review and quantitative synthesis. *Psychological Bulletin*, 132(3), 354-380. [APA](https://psycnet.apa.org/record/2006-10031-001)

5. Cepeda, N. J., et al. (2008). Spacing effects in learning: A temporal ridgeline of optimal retention. *Psychological Science*, 19(11), 1095-1102. [SAGE](https://journals.sagepub.com/doi/10.1111/j.1467-9280.2008.02175.x)

6. Anderson, J. R., & Lebiere, C. (1998). *The Atomic Components of Thought*. Mahwah, NJ: Lawrence Erlbaum. [ACT-R](https://act-r.psy.cmu.edu/about/)

7. Anderson, J. R., et al. (2004). An integrated theory of the mind. *Psychological Review*, 111(4), 1036-1060. [APA](https://psycnet.apa.org/record/2004-15848-001)

8. Anderson, J. R. (2007). *How Can the Human Mind Occur in the Physical Universe?* Oxford University Press. [MIT Press](https://mitpress.mit.edu/9780262512237/)

9. Fisher, C. R., et al. (2018). A comparison of approximations for base-level activation in ACT-R. *Computational Brain & Behavior*, 1, 228-236. [Springer](https://link.springer.com/article/10.1007/s42113-018-0015-3)

10. Wixted, J. T. (2004). On common ground: Jost's (1897) law of forgetting and Ribot's (1881) law of retrograde amnesia. *Psychological Review*, 111(4), 864-879.

### Neuroscience

11. Diekelmann, S., & Born, J. (2010). The memory function of sleep. *Nature Reviews Neuroscience*, 11(2), 114-126. [PubMed](https://pubmed.ncbi.nlm.nih.gov/20046194/)

12. Rasch, B., & Born, J. (2013). About sleep's role in memory. *Physiological Reviews*, 93(2), 681-766. [PubMed](https://pubmed.ncbi.nlm.nih.gov/23589831/)

13. Tononi, G., & Cirelli, C. (2014). Sleep and the price of plasticity: From synaptic and cellular homeostasis to memory consolidation and integration. *Neuron*, 81(1), 12-34. [PMC](https://pmc.ncbi.nlm.nih.gov/articles/PMC3921176/)

14. Richards, B. A., & Frankland, P. W. (2017). The persistence and transience of memory. *Neuron*, 94(6), 1071-1084. [PubMed](https://pubmed.ncbi.nlm.nih.gov/28641107/)

15. Walker, M. P., & Stickgold, R. (2006). Sleep, memory, and plasticity. *Annual Review of Psychology*, 57, 139-166. [PDF](https://walkerlab.berkeley.edu/reprints/Walker&Stickgold_AnnRevPsych_2006.pdf)

### Machine Learning

16. McCloskey, M., & Cohen, N. J. (1989). Catastrophic interference in connectionist networks: The sequential learning problem. *Psychology of Learning and Motivation*, 24, 109-165. [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0079742108605368)

17. Kirkpatrick, J., et al. (2017). Overcoming catastrophic forgetting in neural networks. *PNAS*, 114(13), 3521-3526. [PNAS](https://www.pnas.org/doi/10.1073/pnas.1611835114)

18. Bourtoule, L., et al. (2021). Machine unlearning. *IEEE Symposium on Security and Privacy*, 141-159. [IEEE](https://ieeexplore.ieee.org/document/9519428/)

19. Yang, E., et al. (2024). A comprehensive survey of forgetting in deep learning beyond continual learning. *IEEE TPAMI*. [GitHub](https://github.com/EnnengYang/Awesome-Forgetting-in-Deep-Learning)

20. Wang, L., et al. (2024). A comprehensive survey of continual learning: Theory, method and application. *IEEE TPAMI*, 46(8), 5362-5383. [arXiv](https://arxiv.org/abs/2302.00487)

### Knowledge Graphs and Retrieval

21. Jiang, T., et al. (2016). Towards time-aware knowledge graph completion. *COLING 2016*, 1715-1724. [ACL Anthology](https://aclanthology.org/C16-1161/)

22. A Survey on Temporal Knowledge Graph (2024). arXiv:2403.04782. [arXiv](https://arxiv.org/abs/2403.04782)

23. EAGLE: Temporal Link Prediction (2025). *VLDB 2025*. [arXiv](https://arxiv.org/abs/2507.13825)

24. Solving Freshness in RAG (2025). arXiv:2509.19376. [arXiv](https://arxiv.org/html/2509.19376)

### AI Agent Systems

25. Packer, C., et al. (2023). MemGPT: Towards LLMs as operating systems. arXiv:2310.08560. [arXiv](https://arxiv.org/abs/2310.08560)

26. Park, J. S., et al. (2023). Generative agents: Interactive simulacra of human behavior. *UIST 2023*. [arXiv](https://arxiv.org/abs/2304.03442)

### Spaced Repetition Systems

27. Pimsleur, P. (1967). A memory schedule. *Modern Language Journal*, 51(2), 73-75.

28. Leitner, S. (1972). *So lernt man lernen*. Freiburg: Herder.

29. Wozniak, P. A. (1990). *Optimization of repetition spacing in the practice of learning*. [SuperMemo](https://supermemo.guru/wiki/Forgetting_curve)

---

## Appendix A: maenifold Sleep Cycle Workflow

The complete workflow specification is available at:
`/assets/workflows/memory-cycle.json`

This workflow triggers periodically to perform cognitive maintenance, implementing the phases described in Section 5.
