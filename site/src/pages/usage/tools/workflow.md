---
layout: ../../../layouts/DocsLayout.astro
---

# Workflow

Execute structured workflow(s) for systematic problem-solving and complex orchestration.
  This tool runs predefined workflows that guide you through proven methodologies with embedded sequential thinking, multi-agent coordination, and sophisticated process control.
  Each workflow provides structured steps, tool hints, reasoning requirements, and quality gates for specific problem types.
  REQUIRES [[concepts]] in responses to build connections in the knowledge graph!

## When to Use This Tool
  - Complex multi-step processes requiring orchestration
  - Following established methodologies (design thinking, agile, SLC development)
  - Multi-agent coordination with parallel execution waves
  - Systematic analysis requiring structured approaches with embedded thinking
  - Quality-gated processes with validation checkpoints
  - Sprint execution with RTM (Requirements Traceability Matrix) validation
  - Team collaboration using shared frameworks and workflows
  - When you need proven workflows with embedded sequential thinking vs pure free-form exploration

  Key features:
  - 42+ predefined workflows covering various domains and methodologies
  - **Embedded sequential thinking** - workflows specify when to use SequentialThinking tool
  - Multi-agent coordination with parallel task dispatch
  - Queue-based execution - run multiple workflows in sequence
  - Session persistence across workflow steps with SequentialThinking integration
  - Tool hints and orchestration metadata for each step
  - Quality gates, guardrails, and escape hatches
  - RTM validation for sprint workflows
  - Git integration and tracking for development workflows
  - Enhanced thinking requirements with reasoning effort control
  - Process control with stop conditions and next actions
  - Integration with Maenifold ecosystem (CodeNav, SearchMemories, WriteMemory, etc.)

  Workflow orchestration capabilities:
  - Wave-based execution (discovery wave, validation wave, implementation wave)
  - Parallel agent dispatch (coding-agent, test-agent, desloppification-agent)
  - Cross-tool coordination (CodeNav + SequentialThinking + WriteMemory)
  - Git workflow integration with branch management
  - Requirements traceability with RTM tracking
  - Quality validation with build/test gates
  - Retrospective analysis with pattern extraction

  Available workflow types include:
  - Development methodologies (agentic-dev, agentic-slc sprint orchestration)
  - Design frameworks (design-thinking, lean-startup)
  - Analysis methodologies (critical-thinking, strategic-thinking)
  - Reasoning approaches (deductive, inductive, abductive)
  - Creative processes (divergent-thinking, oblique-strategies)
  - Collaboration methods (world-cafe, parallel-thinking)
  - Software development (agile, sdlc)

  Parameters explained:
  - sessionId: Continue existing workflow session (maintains state across complex orchestration)
  - workflowId: Start new workflow(s) - single ID or JSON array for multi-workflow execution
  - response: Your response to current workflow step - MUST include [[concepts]]
  - thoughts: Meta-observations about workflow progress and orchestration insights
  - status: Set to 'completed' or 'cancelled' to end session
  - view: Display current queue status and orchestration progress
  - append: Add workflow(s) to existing session queue for extended orchestration

  Workflow vs Sequential relationship:
  - **Workflow embeds Sequential**: Many workflow steps specify `🧠 USE SEQUENTIAL THINKING TOOL`
  - **Workflow orchestrates**: Coordinates multiple tools, agents, and thinking sessions
  - **Sequential executes**: Provides iterative thinking within workflow steps
  - **Use Workflow**: For complex orchestrated processes with multiple phases
  - **Use Sequential**: For focused iterative thinking within a single problem space
  - **Combined power**: Workflows specify when Sequential thinking is required with estimated thoughts, focus areas, and reasoning effort

  Advanced orchestration features:
  - ToolHints metadata specifying required tools and operations
  - Reasoning effort classification (low/medium/high)
  - Stop conditions and quality gates
  - Guardrails preventing scope creep and ensuring quality
  - Enhanced thinking requirements with systematic validation
  - Multi-agent collaboration patterns with shared sessions
  - Git discipline and commit tracking
  - RTM (Requirements Traceability Matrix) compliance
  - Escape hatches for ambiguous requirements

  You should:
  1. Use ListWorkflows to see available orchestrated processes
  2. Choose appropriate workflow(s) for your problem complexity
  3. Follow embedded SequentialThinking requirements when specified
  4. Provide thoughtful responses with [[concepts]] at each orchestration step
  5. Complete workflows fully rather than abandoning mid-orchestration
  6. Use multi-workflow approaches for comprehensive systematic analysis
  7. Respect tool hints and quality gates built into workflow steps
  8. Leverage Maenifold ecosystem integration (CodeNav, SearchMemories, etc.)

  Maenifold Integration:
  - Workflows coordinate Maenifold's full tool ecosystem
  - SearchMemories and WriteMemory for knowledge graph integration
  - BuildContext for concept relationship exploration
  - Multi-agent dispatch with Task tool coordination
  - Git workflow management and tracking
  - RTM-driven development with traceability validation