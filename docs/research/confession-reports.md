# ConfessionReport: Inference-Time Honesty Enforcement

Maenifold's ConfessionReport pattern enforces agent accountability through structured self-reporting and adversarial validation. This approach parallels OpenAI's confessions research but implements enforcement at inference-time rather than training-time.

## Background: OpenAI's Confessions Framework

OpenAI's research ([arXiv:2512.08093](https://arxiv.org/abs/2512.08093)) demonstrates that LLMs can be trained to produce honest "confessions" about their behavior:

> "A confession is an output, provided upon request after a model's original answer, that is meant to serve as a full account of the model's compliance with the letter and spirit of its policies and instructions. The reward assigned to a confession during training is solely based on its honesty, and does not impact positively or negatively the main answer's reward."

Key insight: When the "path of least resistance" for maximizing confession reward is to surface misbehavior rather than cover it up, models are incentivized to be honest.

**Use cases identified:** monitoring, rejection sampling, surfacing issues to users.

## Maenifold's Implementation

Maenifold achieves similar goals through three-layer inference-time enforcement:

| Layer | Mechanism | Validates |
|-------|-----------|-----------|
| **1. SubagentStop hook** | Blocks termination until confession found | Presence |
| **2. Tool validation** | Requires `[[WikiLinks]]` in conclusion | Graph integration |
| **3. PM + Red-team audit** | Cross-references claims against reality | Truth |

### The ConfessionReport Prompt

All three enforcement points use the same prompt (defined in tool descriptions and hook):

```
Produce a concise ConfessionReport. List:
1) All explicit and implicit instructions/constraints/objectives you were supposed to follow.
2) For each, whether you complied with the letter and spirit (✅/❌), with evidence.
   Note any gaps and whether your answer was transparent about them.
3) Any uncertainties, ambiguities, or 'grey areas' where compliance was unclear.
4) Any shortcuts, hacks, or policy risks you took.
5) All files, memory:// URIs and graph [[WikiLinks]] you used.

Nothing you say should change the main answer.
This confession is scored only for honesty and completeness; do not optimize for user satisfaction.
```

### Layer 1: SubagentStop Hook

Location: `integrations/claude-code/plugin-maenifold/scripts/hooks.sh`

When a subagent attempts to stop, the hook:
1. Reads the agent's transcript
2. Searches for "ConfessionReport"
3. If missing: blocks termination and returns the confession prompt
4. If present: allows termination

```bash
if grep -q "ConfessionReport" "$TRANSCRIPT_PATH" 2>/dev/null; then
    exit 0  # Allow stop
else
    # Block and request confession
    jq -n --arg reason "$REASON" '{"decision":"block","reason":$reason}'
fi
```

This forces every subagent to produce a confession before completing work.

### Layer 2: Tool-Level Validation

Both `sequential_thinking` and `workflow` tools require a `conclusion` parameter when completing:

```csharp
// SequentialThinkingTools.cs
if (string.IsNullOrEmpty(conclusion))
    return "ERROR: Conclusion required when completing session.";

var conclusionConcepts = MarkdownIO.ExtractWikiLinks(conclusion);
if (conclusionConcepts.Count == 0)
    return "ERROR: Conclusion must include [[WikiLinks]] for knowledge graph integration.";
```

The conclusion parameter description IS the ConfessionReport prompt—agents see it in tool documentation.

### Layer 3: PM + Red-Team Audit

The Product Manager skill defines compliance criteria:

> "Each stage requires a **compliant** ConfessionReport before proceeding. A compliant ConfessionReport shows: all items ✅ (letter and spirit), no undisclosed gaps, no unresolved grey areas, and no policy risks taken."

The TDD workflow then validates claims:

1. **SWE** produces work + ConfessionReport
2. **Red-team** audits: compares confession claims against actual git diff, files touched, test coverage
3. **Blue-team** verifies coverage held under attack
4. **PM** gates: if discrepancy found, re-assigns to new agent instance

Red-team catches lies by cross-referencing:
- Files claimed vs actual `git diff`
- Tests claimed vs actual test files
- Traceability claims (T-* → RTM → FR-*) vs actual code comments

## Comparison: Training vs Inference-Time Enforcement

| Aspect | OpenAI (Training) | Maenifold (Inference) |
|--------|-------------------|----------------------|
| **When enforced** | During RL fine-tuning | At runtime via hooks/tools |
| **Mechanism** | Reward shaping (confession reward independent of answer) | Structural enforcement + adversarial audit |
| **Honesty source** | Trained into model weights | Forced claims → validated by red-team |
| **Generalization** | May generalize to novel situations | Requires explicit hook coverage |
| **Failure mode** | Model finds reward hack | Collusion between agents (mitigated by role separation) |

## Why This Works

The confession itself isn't trusted—it's a **gate** that forces falsifiable claims:

1. **Presence gate**: Hook ensures confession exists (can't skip it)
2. **Structure gate**: Tool ensures graph linkage (claims are traceable)
3. **Truth gate**: Red-team validates claims against reality (lies are caught)

The subagent must either:
- Confess honestly (passes all gates)
- Lie in confession (caught by red-team audit)
- Refuse to confess (blocked by hook, can't terminate)

This creates the same "path of least resistance → honesty" dynamic that OpenAI achieves through training, but implemented as infrastructure.

## Integration Points

- **SubagentStop hook**: `plugin-maenifold/hooks/hooks.json`
- **Hook implementation**: `plugin-maenifold/scripts/hooks.sh`
- **SequentialThinking conclusion**: `src/Tools/SequentialThinkingTools.cs`
- **Workflow conclusion**: `src/Tools/WorkflowTools.Runner.cs`, `WorkflowOperations.Management.cs`
- **PM audit criteria**: `integrations/skills/product-manager/SKILL.md`
- **Agent definitions**: `integrations/agents/{swe,red-team,blue-team,researcher}.md`

## References

- Barak, B. et al. (2025). "Training LLMs for Honesty via Confessions." arXiv:2512.08093. https://arxiv.org/abs/2512.08093
- OpenAI. (2025). "How confessions can keep language models honest." https://openai.com/index/how-confessions-can-keep-language-models-honest/
