# Adopt

Loads [[role]], [[color]], or [[perspective]] JSON configuration from `~/maenifold/assets/` to enable specialized thinking modes.

## Parameters

- `type` (string, required): Asset type: `"role"`, `"color"`, or `"perspective"`
- `identifier` (string, required): Asset filename without `.json` extension

## Returns

Raw JSON configuration with personality, principles, approach, checklists, and anti-patterns.

## Example

```json
{
  "type": "role",
  "identifier": "product-manager"
}
```

Returns SLC framework configuration: Simple, Lovable, Complete principles with customer-focused evaluation criteria.

## Available Assets

### Roles (Professional Domains)
- `architect` - System design, scalability, patterns
- `product-manager` - Customer delight, SLC framework
- `engineer` - Implementation, code quality
- `researcher` - Evidence, investigation, hypothesis testing
- `writer` - Documentation, communication
- `red-team` - Adversarial testing, vulnerability discovery
- `blue-team` - Defense, security hardening
- `codex-specialist` - GPT-5-Codex prompt optimization

### Colors (Six Thinking Hats)
- `white` - Facts, data (neutral)
- `red` - Emotions, intuition (no justification)
- `black` - Risks, problems (critical)
- `yellow` - Benefits, opportunities (optimistic)
- `green` - Creativity, alternatives (generative)
- `blue` - Process control, orchestration (meta)
- `gray` - Analysis, assessment (neutral) - maenifold extension

### Perspectives (Analytical Lenses)
- `critical-analysis` - Skeptical, rigorous examination
- `creative-ambiguity` - Exploratory, possibility-focused
- `evidential-thinking` - Data-driven, proof-based
- `factual-clarity` - Precise, objective
- `emotional-expression` - Feelings, subjective
- `relational-expression` - Connections, relationships
- `conceptual-synthesis` - Integration, holistic
- `hierarchical-precision` - Structure, taxonomy
- `aspectual-analysis` - Multi-faceted examination
- `positive-framing` - Constructive, opportunity-focused
- `skeptical-inquiry` - Questioning assumptions
- `process-control` - Workflow orchestration

## Common Patterns

### Six Thinking Hats Sequence
```bash
Blue → White → Red → Green → Yellow → Black → Blue
Process → Facts → Feelings → Ideas → Benefits → Risks → Synthesis
```

### Multi-Role Code Review
```bash
Engineer → Architect → Red Team → Blue Team
Implementation → Design → Attack → Defend
```

### Sequential Thinking with Role
```json
{
  "sessionId": "design-review",
  "response": "Adopting [[architect]] role to evaluate system design..."
}
```

## Integration

- **SequentialThinking**: Adopt roles during structured analysis
- **Workflow**: Orchestrate role sequences automatically
- **WriteMemory**: Document insights from different thinking modes

## Troubleshooting

**Error: "Invalid type 'hat'"**
→ Use `type="color"` for Six Thinking Hats

**Error: "Asset not found: role/pm"**
→ Use exact identifier: `"product-manager"` not `"pm"`

**Behavior unchanged after adoption**
→ Expected - tool returns configuration, agent must apply it
