---
name: ma:adopt
description: Loads role, color, or perspective JSON configuration to enable specialized thinking modes
---

Loads role, color, or perspective JSON configuration to enable specialized thinking modes.

Assets are loaded from two locations (runtime takes precedence):
1. `$MAENIFOLD_ROOT/assets/` - User-created assets at runtime
2. Built-in assets bundled with maenifold

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

## Built-in Assets

### Roles (16 Professional Domains)
- `architect` - System design, scalability, patterns
- `blue-team` - Defense, security hardening
- `cfo` - Financial strategy, fiscal discipline
- `eda-architect` - Event-driven architecture design
- `eda-platform-engineer` - EDA platform implementation
- `engineer` - Implementation, code quality
- `finops-practitioner` - Cloud financial operations
- `ftk-agent` - First-time knowledge acquisition
- `mcp-specialist` - Model Context Protocol expertise
- `product-manager` - Customer delight, SLC framework
- `prompt-engineer` - Prompt design and optimization
- `prompt-engineer-codex` - Codex-specific prompting
- `prompt-engineer-gpt5` - GPT-5 prompt optimization
- `red-team` - Adversarial testing, vulnerability discovery
- `researcher` - Evidence, investigation, hypothesis testing
- `writer` - Documentation, communication

### Colors (7 Six Thinking Hats)
- `white` - Facts, data (neutral)
- `red` - Emotions, intuition (no justification)
- `black` - Risks, problems (critical)
- `yellow` - Benefits, opportunities (optimistic)
- `green` - Creativity, alternatives (generative)
- `blue` - Process control, orchestration (meta)
- `gray` - Analysis, assessment (maenifold extension)

### Perspectives (12 Language Modes)
- `en` - English
- `de` - German
- `es` - Spanish
- `fr` - French
- `it` - Italian
- `ja` - Japanese
- `ko` - Korean
- `pt` - Portuguese
- `ru` - Russian
- `tr` - Turkish
- `zh` - Chinese
- `ar` - Arabic

## Custom Assets

Create custom assets in `$MAENIFOLD_ROOT/assets/{roles,colors,perspectives}/`:

```json
// $MAENIFOLD_ROOT/assets/roles/my-custom-role.json
{
  "id": "my-custom-role",
  "name": "My Custom Role",
  "description": "Custom role for specific use case",
  "principles": ["principle1", "principle2"],
  "approach": "How to apply this role"
}
```

Runtime assets override built-in assets with the same identifier.

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
  "response": "Adopting architect role to evaluate system design..."
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
