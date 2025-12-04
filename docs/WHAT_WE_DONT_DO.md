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
- ❌ Score memory importance or relevance
- ❌ Implement memory compression or archival
- ❌ Create memory hierarchies or priorities
- ❌ Auto-delete "old" or "unimportant" memories
- ❌ Build memory indexes beyond basic search

**Why:** The LLM decides what's important, how to organize, what to remember.

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

### 2. **Memory Search Ranking**
"We should rank search results by relevance/recency/importance!"

**No.** We return results ordered by modification time. The LLM decides what's relevant in its current context.

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

## A Living Document

This document will grow as we resist new features. Each addition to this list represents a victory for Ma - another space preserved for intelligence to operate freely.

Remember: **The absence IS the feature.**