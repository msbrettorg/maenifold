# maenifold-product-team

An opinionated product team workflow built on maenifold primitives.

## Prerequisites

1. The `maenifold` binary must be in your PATH
2. **Install the base maenifold plugin first:**
   ```bash
   claude plugin add /path/to/plugin-maenifold
   ```

## Installation

```bash
claude plugin add /path/to/plugin-product-team
```

## What's Included

### Agents

| Agent | Purpose |
|-------|---------|
| **swe** | Software engineering implementation |
| **researcher** | Deep research and investigation |
| **red-team** | Adversarial security analysis |
| **blue-team** | Defensive security measures |

### Hooks

| Event | Purpose |
|-------|---------|
| `PreToolUse` (Task) | Augments task prompts with knowledge graph context |
| `SubagentStop` | Captures agent outputs back to knowledge graph |

### Skills

- **product-manager** - Product management workflow guidance

## How It Works

This plugin demonstrates how to compose maenifold's primitives into a complete workflow:

1. **Context injection** - Task agents receive relevant knowledge graph context
2. **Knowledge capture** - Agent outputs are written back to the graph
3. **Specialized agents** - Role-specific agents with maenifold awareness

Use this as a reference for building your own maenifold-powered workflows.
