# Task Tracking: GitHub Actions Failure Handling Research

## Task

Research official GitHub Actions documentation for build failure handling and error recovery patterns.

**Requirements:**
- ONLY use official GitHub documentation (docs.github.com)
- Find exact instructions for handling workflow failures
- Get examples of failure notifications, retry logic, error reporting
- Find official patterns for build failure recovery
- Include source URLs for all findings

**Output Required:**
Create a markdown document with:
1. Exact quotes from official docs
2. Workflow YAML examples for failure handling
3. Notification options (GitHub UI, email, etc.)
4. Retry patterns (if officially supported)
5. Source URLs

Write findings to: /Users/brett/src/ma-collective/maenifold/docs/status-tracking/github-actions-failure-handling.md

## Task Restated

I need to research and document official GitHub Actions failure handling capabilities by:
1. Searching ONLY official GitHub documentation (docs.github.com)
2. Finding documented instructions for workflow failure handling
3. Collecting YAML examples showing failure handling patterns
4. Identifying official notification mechanisms
5. Documenting any official retry/recovery patterns
6. Preserving exact quotes and source URLs

**Assumptions:**
- Will use WebFetch/WebSearch to access official GitHub docs
- Will create comprehensive markdown document with findings
- Will not include third-party or community patterns, only official documentation
- Will preserve exact quotes to maintain accuracy per Ma Protocol

**What I will do:**
- Search official GitHub Actions documentation
- Extract exact quotes and examples
- Document all source URLs
- Create structured markdown report

**What I will NOT do:**
- Use non-official sources
- Make assumptions about undocumented features
- Provide examples not from official docs

## Plan

1. Search official GitHub Actions docs for failure handling
2. Search for notification mechanisms
3. Search for retry patterns
4. Extract quotes and examples
5. Compile findings into structured markdown document

## Status

✓ Completed web searches for failure handling documentation
✓ Fetched detailed content from official GitHub docs pages
✓ Extracted exact quotes and YAML examples
✓ Documented notification mechanisms
✓ Found retry/re-run patterns
✓ Compiled all source URLs

Research complete. Creating final findings document...

✓ COMPLETE: Final document created at /Users/brett/src/ma-collective/maenifold/docs/status-tracking/github-actions-failure-handling.md

## Summary

Successfully researched and documented official GitHub Actions failure handling capabilities including:

- Workflow, job, and step-level failure handling (continue-on-error, fail-fast, timeout-minutes)
- Status check functions (success(), failure(), always(), cancelled())
- Job dependencies and failure propagation patterns
- Detection mechanisms (steps context, needs context, job context)
- Official notification options (email, UI, badges, webhooks, Slack)
- Manual re-run capabilities (UI and CLI methods)
- Debug logging and troubleshooting patterns
- workflow_run event for triggering on failures
- Complete YAML examples demonstrating all patterns

All information sourced exclusively from docs.github.com with exact quotes and source URLs preserved per Ma Protocol requirements.
