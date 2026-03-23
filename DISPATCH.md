# Pipeline Dispatch — DEPRECATED

> **This document is deprecated.** Pipeline dispatch is now handled by the `gate` CLI tool and the `gate` skill.

## Quick Reference

- **Status**: `gate status --root .`
- **Advance**: `gate advance <FILE> --root . --stage <STAGE> [--verdict <VERDICT>]`
- **Rebuild cache**: `gate sync --rebuild .`
- **Full process**: See `assets/integrations/skills/gate/SKILL.md`

## History

This file previously documented the raw `sqlite3 pipeline.db` workflow. As of Sprint 53 (FR-41), pipeline state is file-authoritative — markdown task files in `todo/` and `done/` are the source of truth, and `pipeline.db` is a rebuildable cache managed by the `gate` CLI.
