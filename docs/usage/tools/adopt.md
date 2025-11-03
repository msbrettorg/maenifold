# Adopt

Adopt a [[role]], [[color]], or [[perspective]] by loading its JSON configuration from Maenifold's assets directory. This tool enables AI agents to take on specialized thinking modes, [[Six Thinking Hats]] colors, or analytical perspectives by reading structured configuration that defines personality, approach, and evaluation criteria.

## When to Use This Tool

- **Role-Based Thinking**: Adopt specialized professional roles (product-manager, architect, engineer, researcher)
- **Six Thinking Hats**: Use [[De Bono thinking hats]] for structured problem analysis (blue, white, red, green, yellow, black)
- **Perspective Shifts**: Apply analytical lenses (critical-analysis, creative-ambiguity, evidential-thinking)
- **Structured Workflows**: Load role configurations for [[workflow]] or [[sequential-thinking]] sessions
- **Context Switching**: Change thinking mode mid-session for different analysis phases
- **Multi-Agent Coordination**: Different agents adopt different roles for collaborative problem-solving
- **Quality Frameworks**: Apply specialized evaluation criteria from role checklists

## Key Features

- **JSON Configuration Loading**: Reads structured role/color/perspective definitions from assets
- **Three Asset Types**: Supports roles, colors (Six Thinking Hats), and perspectives
- **Rich Metadata**: Each asset includes name, motto, principles, approach, and checklists
- **Validation**: Ensures asset type is valid and file exists before loading
- **Direct JSON Return**: Returns raw JSON configuration for agent consumption
- **No State Modification**: Tool doesn't change agent state - just provides configuration data
- **Asset Library**: Pre-built collection of proven thinking frameworks

## Parameters

| Parameter | Type | Required | Description | Example |
|-----------|------|----------|-------------|---------|
| type | string | Yes | Type of asset to adopt: 'role', 'color', or 'perspective' | "role", "color", "perspective" |
| identifier | string | Yes | Identifier of the asset (filename without .json extension) | "product-manager", "blue", "critical-analysis" |

## Usage Examples

### Adopt Product Manager Role
```json
{
  "type": "role",
  "identifier": "product-manager"
}
```
Loads product manager configuration with SLC (Simple, Lovable, Complete) framework and customer-focused planning.

### Adopt Blue Hat (Six Thinking Hats)
```json
{
  "type": "color",
  "identifier": "blue"
}
```
Adopts Blue Hat thinking - process control, orchestration, and meta-thinking about thinking.

### Adopt Critical Analysis Perspective
```json
{
  "type": "perspective",
  "identifier": "critical-analysis"
}
```
Applies critical analysis lens for rigorous evaluation and skeptical inquiry.

### Adopt Engineer Role
```json
{
  "type": "role",
  "identifier": "engineer"
}
```
Loads software engineer configuration with implementation focus and technical best practices.

### Adopt Red Hat (Six Thinking Hats)
```json
{
  "type": "color",
  "identifier": "red"
}
```
Red Hat thinking - emotions, intuition, and gut reactions without justification.

### Adopt Green Hat (Six Thinking Hats)
```json
{
  "type": "color",
  "identifier": "green"
}
```
Green Hat thinking - creativity, possibilities, alternatives, and new ideas.

## Available Assets

### Roles (Professional Thinking Modes)
- **architect**: System design, architectural patterns, scalability considerations
- **product-manager**: Customer delight, SLC framework (Simple, Lovable, Complete)
- **engineer**: Implementation, code quality, technical execution
- **researcher**: Evidence gathering, systematic investigation, hypothesis testing
- **writer**: Clear communication, documentation, user-facing content
- **red-team**: Adversarial testing, vulnerability discovery, security assessment
- **blue-team**: Defense, security hardening, incident response

### Colors (Six Thinking Hats)
- **white**: Facts, data, information (neutral, objective)
- **red**: Emotions, feelings, intuition (no justification needed)
- **black**: Caution, risks, problems (critical judgment)
- **yellow**: Benefits, optimism, opportunities (positive exploration)
- **green**: Creativity, alternatives, new ideas (generative thinking)
- **blue**: Process control, orchestration (meta-thinking)
- **gray**: Analysis, assessment (neutral evaluation) - Added by maenifold

### Perspectives (Analytical Lenses)
- **critical-analysis**: Skeptical, evaluative, rigorous examination
- **creative-ambiguity**: Exploratory, open-ended, possibility-focused
- **evidential-thinking**: Data-driven, proof-based, verifiable claims
- **factual-clarity**: Precise, unambiguous, objective statements
- **emotional-expression**: Feelings, reactions, subjective experience
- **relational-expression**: Connections, relationships, interactions
- **conceptual-synthesis**: Integration, unification, holistic understanding
- **hierarchical-precision**: Structure, organization, taxonomy
- **aspectual-analysis**: Multi-faceted examination, different angles
- **positive-framing**: Constructive, opportunity-focused perspective
- **skeptical-inquiry**: Questioning assumptions, challenging claims
- **process-control**: Workflow management, orchestration

## Asset JSON Structure

Each asset contains:

### Core Identity
```json
{
  "id": "product-manager",
  "name": "The product manager",
  "emoji": "📊",
  "shortDescription": "Define what we build and why it matters to customers"
}
```

### Personality
```json
{
  "personality": {
    "motto": "Is this Simple, Lovable, and Complete for our customers?",
    "principles": [
      "Simple solutions over complex features—elegance matters",
      "Lovable products that customers genuinely want to use"
    ]
  }
}
```

### Approach (Methodology)
Detailed thinking frameworks, evaluation criteria, and decision-making processes specific to the role/color/perspective.

### Response Style
How the adopted mindset evaluates problems and formulates responses.

### Checklists
Quality gates and validation criteria for the adopted thinking mode.

### Anti-Patterns
What to avoid when operating in this mode.

### Transition Triggers (Roles only)
When to switch to other roles during collaborative work.

## Common Patterns

### Sequential Thinking with Role Adoption
```json
{
  "sessionId": "analysis-session",
  "response": "After adopting [[product-manager]] role, analyzing customer value..."
}
```
Start thinking session by adopting relevant role for structured analysis.

### Six Thinking Hats Workflow
```bash
# Phase 1: Blue Hat - Process design
Adopt type="color" identifier="blue"

# Phase 2: White Hat - Facts
Adopt type="color" identifier="white"

# Phase 3: Red Hat - Gut reactions
Adopt type="color" identifier="red"

# Phase 4: Green Hat - Creative solutions
Adopt type="color" identifier="green"

# Phase 5: Yellow Hat - Benefits analysis
Adopt type="color" identifier="yellow"

# Phase 6: Black Hat - Risk assessment
Adopt type="color" identifier="black"

# Phase 7: Blue Hat - Synthesis
Adopt type="color" identifier="blue"
```

### Role-Based Code Review
```bash
# Phase 1: Engineer reviews implementation
Adopt type="role" identifier="engineer"

# Phase 2: Architect reviews design
Adopt type="role" identifier="architect"

# Phase 3: Red Team attacks security
Adopt type="role" identifier="red-team"

# Phase 4: Blue Team defends
Adopt type="role" identifier="blue-team"
```

### Perspective-Shifting Analysis
```bash
# Critical lens
Adopt type="perspective" identifier="critical-analysis"

# Creative lens
Adopt type="perspective" identifier="creative-ambiguity"

# Evidence lens
Adopt type="perspective" identifier="evidential-thinking"
```

## Integration with Workflows

### Six Thinking Hats Workflow
The [[Workflow]] tool can automatically use Adopt to load color configurations for Six Thinking Hats facilitation.

### Custom Workflows
Workflows can specify role adoption in their steps:
```yaml
- step: adopt_role
  tool: Adopt
  parameters:
    type: role
    identifier: product-manager
```

### Sequential Thinking
Thinking sessions can adopt roles for specialized analysis phases:
```json
{
  "thought": "Adopting [[architect]] role to evaluate system design...",
  "sessionId": "design-review"
}
```

## Related Tools

- **SequentialThinking**: Use adopted roles during structured thinking sessions
- **Workflow**: Workflows can orchestrate role adoption sequences
- **ListMemories**: Discover available role/color/perspective assets
- **WriteMemory**: Document insights gained from different thinking modes

## Troubleshooting

### Error: "Invalid type 'hat'. Must be one of: role, color, perspective"
**Cause**: Used "hat" instead of "color" for Six Thinking Hats
**Solution**: Use type="color" for Six Thinking Hats (blue, white, red, green, yellow, black)

### Error: "Asset not found: role/pm"
**Cause**: Identifier doesn't match any asset filename
**Solution**: Use exact identifier: "product-manager" not "pm", check spelling

### Result: Empty or malformed JSON
**Cause**: Asset file is corrupted or empty
**Solution**: Verify asset file integrity in src/assets/{type}s/{identifier}.json

### Unsure Which Asset to Use
**Cause**: Unclear which role/color/perspective fits the task
**Solution**:
  - Roles: Use for professional domain expertise (engineer, architect, product-manager)
  - Colors: Use for Six Thinking Hats structured analysis
  - Perspectives: Use for analytical lenses (critical, creative, evidential)

### Asset Loaded But Behavior Unchanged
**Cause**: Adopt tool returns configuration but doesn't change agent state
**Solution**: This is expected - agent must interpret and apply the loaded configuration

## Example Adoption Session

### Step 1: Adopt Product Manager Role
```json
{
  "type": "role",
  "identifier": "product-manager"
}
```

### Step 2: Review Configuration
```json
{
  "id": "product-manager",
  "name": "The product manager",
  "personality": {
    "motto": "Is this Simple, Lovable, and Complete for our customers?"
  }
}
```

### Step 3: Apply Framework
Use the SLC (Simple, Lovable, Complete) framework from the role configuration to evaluate product decisions.

### Step 4: Use Checklist
Apply the role's checklist:
- Will customers genuinely love using this?
- Is this the simplest solution that works completely?
- Does this solve the whole customer problem?

## Six Thinking Hats Quick Reference

- **🔵 Blue Hat**: Process control, orchestration, thinking about thinking
- **⚪ White Hat**: Facts, data, information (neutral, objective)
- **🔴 Red Hat**: Emotions, feelings, intuition (no justification needed)
- **⚫ Black Hat**: Caution, risks, problems (critical judgment)
- **🟡 Yellow Hat**: Benefits, optimism, opportunities (positive)
- **🟢 Green Hat**: Creativity, alternatives, new ideas (generative)
- **⚪ Gray Hat**: Analysis, assessment (neutral evaluation) - maenifold extension

## Design Philosophy

### Why JSON Assets?
- **Structured Thinking**: JSON provides clear framework for role definition
- **Reusable Patterns**: Proven thinking modes captured as configuration
- **Composable**: Mix and match roles, colors, perspectives as needed
- **Transparent**: Full configuration visible - no hidden behavior
- **Extensible**: Add new roles/colors/perspectives by adding JSON files

### Ma Protocol Alignment
- **No Magic**: Simple file reading - configuration over code
- **Real Assets**: JSON files are the source of truth
- **Minimal State**: Tool doesn't maintain state - just loads data
- **Single Responsibility**: Loads configuration, nothing more
- **Transparent Operation**: Returns raw JSON for agent interpretation

## Ma Protocol Compliance

Adopt follows Maenifold's Ma Protocol principles:
- **Simplicity**: Direct file reading with validation
- **No Hidden State**: Tool has no persistent state - pure data loading
- **Transparent**: Returns complete configuration for agent to interpret
- **Real Files**: Reads actual JSON assets, no abstractions
- **Minimal Complexity**: Static method, simple parameters, clear errors
- **Extensible**: Add new assets by adding JSON files to assets directory

This tool represents Ma Protocol's principle of **space for structured thinking** - providing frameworks that guide without constraining, supporting emergence of intelligent behavior through well-defined cognitive modes.
