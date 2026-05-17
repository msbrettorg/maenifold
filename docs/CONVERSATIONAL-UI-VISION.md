# Conversational UI Vision for maenifold

## Thesis

[[AI is the new UI]] should not stop at giving the desktop app an interface that speaks [[MCP]] like an AI. The next step is to make [[maenifold]] itself conversational.

This means the product should stop treating conversation as something that happens only inside an [[ACP]] chat session on the [[Agents]] tab. Instead, the app itself should become a shell-native [[conversational-ui]] that users can talk to, type to, and operate through.

The assistant is not a [[product-manager]] agent, not an [[ACP-session]], and not a role-specific worker. It is the app's own multimodal control layer.

## Product Shape

maenifold should gain a persistent shell-level assistant surface, conceptually an [[ambient-copilot]], [[floating-command-window]], or [[conversational-shell]].

This surface should:

- float above all primary workspaces
- remain available while navigating between views
- preserve transcript and context across navigation
- support both voice and keyboard interaction
- speak back to the user
- drive the app on the user's behalf

It is a shell feature, not a tab.

## User Experience Goals

The desired experience is modern, conversational, and multimodal.

The user should be able to:

- speak to maenifold
- type to maenifold
- interrupt and correct it naturally
- see the ongoing transcript in a floating window
- hear spoken responses from maenifold
- ask maenifold to navigate the app
- ask maenifold to perform tasks inside the app

Example interactions:

- "Open Gate."
- "Show only tasks waiting on blue-team."
- "Read the first one."
- "Go to Knowledge and search for AI is the new UI."
- "Open the runner panel."
- "Create a workflow from this description."

The key shift is from "open a chat with an agent inside maenifold" to "talk to maenifold itself."

## Outcome-First Interaction

The assistant should be built around [[outcome-first-interaction]], not control-by-control instruction following.

The user should not need to tell the system what to click, which tab to select, or which button sequence to execute. Those details are implementation mechanics that belong to the assistant, not the user.

The user should be able to describe what they want, even imprecisely:

- "Show me the tasks that are stuck."
- "Open the thing we were just looking at."
- "Find the workflow about research and let me edit it."
- "Take me to the place where I can fix voice settings."

The assistant should translate these fuzzy requests into the product's actual capabilities.

That means the conversational layer must:

- infer intent from poor or incomplete phrasing
- map user goals onto existing app commands and MCP-backed capabilities
- choose the right sequence of navigation and action steps implicitly
- ask clarifying questions only when ambiguity is material

The product goal is not "voice control for the UI." The product goal is "intent translation into visible action."

## Visual Confirmation Matters

People are visual. The assistant should not disappear into hidden execution and then merely report what it did.

When the user asks for something, the system should produce visible evidence in the product shell:

- navigate to the relevant workspace
- open the relevant panel or object
- select the relevant item
- reveal the resulting state change in the visible UI
- keep the floating transcript in sync with what happened

The user should be able to both **hear** the response and **see** the result.

This is essential to trust. The assistant is not only a conversational endpoint. It is a guide and operator whose work should remain visually inspectable.

## Implicit Execution, Explicit Results

The interaction model should be:

- **implicit mechanics** — the assistant decides how to operate the product
- **explicit results** — the user sees the destination, the selection, the changed state, and the transcript explanation

The user expresses intent.
The assistant chooses the path.
The UI shows the outcome.

That is the correct expression of [[AI is the new UI]] for a visual desktop product.

## Floating Window

The assistant needs a visible, persistent floating transcript surface.

Requirements:

- draggable
- pinnable
- collapsible to a compact affordance
- restorable without losing context
- visible across all top-level workspaces
- able to show what it heard, what it understood, what it is doing, and what happened

This surface should function like a cross between a command palette, a transcript window, and a live operator console.

## Voice Is Product, Not Add-On

The existing [[Handy]] integration proves one narrow part of the problem: getting [[speech-to-text]] into the app via a companion process.

That is useful, but it is not the product.

The full conversational UI requires:

- speech input
- typed input
- transcript ownership
- spoken output via [[text-to-speech]]
- turn-taking and interruption handling
- action execution against the app
- clear progress and result reporting

[[Handy]] may be reused temporarily as an implementation choice for speech input, but it should not define the architecture. It is optional infrastructure, not the identity of the feature.

## Core Architectural Principle

Do not implement this as brittle screen automation.

Instead, give the app a first-class [[app-control-bus]] with structured commands and observable results.

Examples:

- `navigate.toTab("Gate")`
- `gate.selectTask("T-204.0")`
- `knowledge.search("AI is the new UI")`
- `pipelines.openRunner()`
- `settings.openSection("voice")`
- `shell.focusComposer()`

The conversational assistant should drive maenifold through these commands, not through scraping the visible UI and pretending to be a human clicking random controls.

This preserves:

- reliability
- auditability
- safety
- testability
- explainability

It also keeps the design aligned with the repo's [[MCP]]-first and [[AI is the new UI]] doctrine.

## Three-Layer Model

### 1. Perception and Dialogue

This layer handles:

- speech capture
- typed input
- transcript state
- turn state
- spoken output
- interruption and barge-in
- listening, speaking, and idle indicators

### 2. Intent and Planning

This layer maps user input into structured actions:

- [[navigational-intents]]
- [[domain-intents]]
- clarification prompts
- readback requests
- tool invocations

This layer does not have to be an [[LLM]]. It can be:

- deterministic for known shell commands
- grammar-based for high-confidence navigation and control
- model-assisted only where ambiguity or synthesis is required

The right strategy is commands first, model second.

### 3. Actuation

This layer performs actions through:

- the [[app-control-bus]] for shell and UI actions
- the existing [[MCP]] surface for backend and domain operations

This lets the assistant both navigate the app and invoke maenifold capabilities without introducing a separate backend API just for the UI.

## Safety Model

The assistant should operate under explicit action classes.

### Navigational actions

These may auto-run:

- switching tabs
- opening and closing panels
- focusing controls
- reading current visible state
- local filtering and search

### Non-destructive operational actions

These should use lightweight inline confirmation or a configurable trust preference:

- starting a pipeline run
- creating a draft workflow
- opening a new session
- invoking non-destructive tools

### Destructive or state-changing actions

These require explicit confirmation:

- deletion
- mutation of knowledge
- `maenifold.gate` pass or fail actions
- cancelling runs
- workflow edits with lasting effect

The assistant should be fluid, but never hidden or magical in the wrong way.

## Relationship to Existing Surfaces

This new assistant does not replace the existing workspaces. It complements them.

- [[Agents]] remains the home for direct [[ACP]] conversations.
- [[Pipelines]] remains the graph execution and inspection surface.
- [[Gate]] remains the task tracker.
- [[Knowledge]] remains the notes and concept graph surface.
- [[Workflows]] remains the asset and method library.

The conversational shell becomes the front door and roaming operator across all of them.

## MVP Definition

The MVP should establish the product shape, not solve every speech or autonomy problem.

### MVP must include

- always-available floating transcript window
- typed input
- push-to-talk speech input
- spoken responses
- shell navigation commands
- readback of current view or selected object
- permissioned execution of structured app actions

### MVP should not require

- continuous ambient listening
- fully autonomous task completion
- computer-use style DOM scraping
- deep reasoning as a prerequisite for every command
- commitment to any one speech backend

## Implementation Path

### Phase 1: Conversational shell MVP

Ship the floating shell, transcript, text input, push-to-talk, spoken output, and shell navigation.

### Phase 2: App control bus

Expose top-level app commands for navigation, selection, focus, and panel control.

### Phase 3: MCP-integrated operator

Let the shell invoke existing domain capabilities such as memory search, graph context, `maenifold.gate` status, pipeline actions, and workflow discovery.

### Phase 4: Adaptive intent engine

Add more flexible parsing of natural language, references like "that task" or "this pipeline," and optional model-backed disambiguation.

### Phase 5: Ambient mode

Optionally add wake-word, detached mini-window behavior, continuous listening, and cross-window presence.

## Product Statement

The strongest expression of the vision is simple:

**Stop putting AI in a tab. Make the app itself conversational.**

Or, more precisely:

**maenifold should gain a shell-native, multimodal conversational UI that can speak with the user and operate the product through the same structured capability surface the rest of the app already uses.**

## Grounding

- `memory://architecture/ai-is-the-new-ui-mcp-as-common-interface`
- `memory://directives/acpexec-as-unified-execution-path` (superseded for PM/task execution; ACP UI runtime only)
- `memory://research/agentic-user-interfaces-where-the-agent-draws-the-ui`
- `memory://research/agentic-ui-convergence-bets-for-maenifold-april-2026`
- `memory://sprints/204/sprint-204-handy-adoption-research`
- `memory://sprints/204/sprint-204-complete-handy-voice-input-epic`
- `memory://reviews/maenifoldrs-product-state-summary-20260410`
- `docs/README.md`
- `docs/UAT-PLAN.md`
- sequential thinking session `session-1777146952658-76173`
