# What Ma Intentionally Does NOT Do

This document celebrates the features we've resisted adding. Each absence creates space (Ma) for AI to work more effectively.

## Core Principles We Don't Violate

### 1. NO FAKE AI
We don't:
- ❌ Add retry logic that tries different approaches
- ❌ Implement fallback strategies when operations fail
- ❌ Create "smart" error recovery mechanisms
- ❌ Build adaptive behavior based on past failures
- ❌ Make decisions about what the user "probably meant"

**Why:** Every decision we make removes a decision the LLM could make. The LLM has context we don't.

### 2. NO UNNECESSARY ABSTRACTIONS
We don't:
- ❌ Create a "MemoryInterface" when we just need MemoryDatabase
- ❌ Build factory patterns for simple object creation
- ❌ Add dependency injection frameworks
- ❌ Create abstract base classes for single implementations
- ❌ Hide simple operations behind complex APIs

**Why:** Abstractions create cognitive load. Simplicity creates understanding.

### 3. NO PERFORMANCE OPTIMIZATIONS (without measurement)
We don't:
- ❌ Add caching layers "just in case"
- ❌ Implement connection pooling by default
- ❌ Create background workers for "faster processing"
- ❌ Batch operations that work fine individually
- ❌ Optimize code that isn't proven slow

**Why:** Premature optimization removes flexibility. The LLM might need that "inefficient" behavior.

## Specific Things We Don't Do

### In Memory Management
We don't:
- ❌ Automatically categorize or tag memories
- ❌ Judge the *content quality* of memories
- ❌ Implement memory compression or archival
- ❌ Auto-delete memories — ever
- ❌ Build memory indexes beyond basic search

We *do* apply temporal signals:
- ✅ **Decay weighting** — time-based deprioritization (not deletion) via power-law decay
- ✅ **Access boosting** — deliberate reads reset decay clocks
- ✅ **Tiered half-lives** — episodic memories decay faster than procedural ones
- ✅ **Cognitive sleep cycle** — periodic consolidation of episodic → semantic knowledge

**Why the evolution:** This document originally said "don't score memory importance." Then the [decay research](research/decay-in-ai-memory-systems.md) happened — 29 citations from Ebbinghaus through Richards & Frankland proving that memory systems without forgetting become *worse* over time. The engineering demanded change.

**The distinction we preserve:** Decay scores *freshness*, not *worth*. The system has opinions about when you last used something, not about whether it was good. The LLM still decides what's important. The system decides what's recent.

### In Tool Design
We don't:
- ❌ Combine tools that could be separate
- ❌ Add optional parameters for "convenience"
- ❌ Create tool aliases or shortcuts
- ❌ Build meta-tools that call other tools
- ❌ Implement tool versioning or deprecation

**Why:** Simple tools with single purposes are easier to understand and compose.

### In Error Handling
We don't:
- ❌ Catch and recover from errors automatically
- ❌ Retry failed operations with modifications
- ❌ Provide suggested fixes for errors
- ❌ Hide error details for "cleaner" output
- ❌ Create error categorization systems

**Why:** Errors are information. The LLM needs all of it to make decisions.

### In Configuration
We don't:
- ❌ Validate configuration beyond basic types
- ❌ Provide configuration wizards or generators
- ❌ Auto-migrate old configurations
- ❌ Suggest "better" configuration values
- ❌ Create configuration inheritance hierarchies

**Why:** Configuration is user space. We provide mechanism, not policy.

### In Testing
We don't:
- ❌ Mock external dependencies
- ❌ Create test fixtures that don't match reality
- ❌ Use in-memory replacements for real systems
- ❌ Generate random test data
- ❌ Clean up test artifacts automatically

**Why:** Real tests with real systems find real problems. Test artifacts help debugging.

## Features We've Explicitly Rejected

### 1. **Plugin System**
"We should add a plugin system so users can extend functionality!"

**No.** Users can modify the source. A plugin system adds complexity and constraints. The absence of a plugin API means users can change anything.

### 2. **Memory Search Ranking** *(evolved)*
"We should rank search results by relevance/recency/importance!"

**Partially.** We now apply decay weighting to search and context results — recent and frequently-accessed memories rank higher. But we don't evaluate content quality or topical relevance. Hybrid search (semantic + full-text) handles relevance; decay handles freshness. The LLM still decides what matters in its current context.

### 3. **Automatic Backup**
"We should automatically backup the memory directory!"

**No.** Backup is a user concern. We don't make assumptions about their backup strategy.

### 4. **Rate Limiting**
"We should add rate limiting to prevent abuse!"

**No.** This is a local tool. If users want to abuse their own system, that's their choice.

### 5. **Telemetry**
"We should add anonymous usage statistics!"

**No.** Not even anonymous. Not even opt-in. The absence of telemetry is a feature.

### 6. **Update Checker**
"We should check for updates and notify users!"

**No.** Users can check for updates when they want to. We don't phone home.

## The Power of Absence

Each missing feature creates space for:
- User customization without fighting defaults
- LLM decision-making without constraints
- Simple debugging without abstraction layers
- Clear understanding without documentation
- Trust without privacy concerns

## What We Changed Our Minds About

間 is a design instinct, not a design law. Some absences had to give way when the evidence demanded it:

| Original Position | What Changed | Why |
|---|---|---|
| Don't score memory importance | Added decay weighting | 29-citation research paper proved unmanaged memory degrades retrieval quality |
| Don't create memory hierarchies | Added tiered decay (episodic/semantic/procedural) | Different memory types have different shelf lives — neuroscience is clear on this |
| Return results by modification time only | Added decay-weighted ranking | Without freshness signals, old noise drowns recent signal (context rot) |

**What we preserved through the change:**
- Never delete memories (decay deprioritizes, doesn't destroy)
- Never judge content quality (decay measures freshness, not worth)
- Never make decisions the LLM should make (the LLM decides what's important; the system decides what's recent)

Changing our minds isn't a violation of Ma. Refusing to change when the evidence demands it would be.

## A Living Document

This document will grow as we resist new features — and as we honestly acknowledge when principled resistance meets empirical reality. Each absence on this list represents a victory for Ma. Each evolution represents Ma maturing.

Remember: **The absence IS the feature. And knowing when to break the rule IS the wisdom.**