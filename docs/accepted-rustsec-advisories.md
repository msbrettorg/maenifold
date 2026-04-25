# Accepted RustSec Advisories

This document records RustSec advisories that are currently accepted because they
enter the workspace only through transitive dependencies with no compatible
upstream upgrade path available.

Accepted on: 2026-04-25

Review cadence:

- Re-check quarterly.
- Re-check immediately when any parent crate listed below releases an update.
- Re-run `cargo audit` before changing or renewing an accepted risk.

## Accepted transitive warnings

### RUSTSEC-2024-0384 — `instant` 0.1.13

- Crate: `instant`
- Reason: unmaintained crate advisory.
- Severity: low; no known vulnerability is identified by the advisory.
- Dependency chain:
  - `instant 0.1.13`
  - `notify-types 1.0.1`
  - `notify 7.0.0`
  - `maenifold-core`, `maenifold-gate`
- Upstream blocker:
  - `notify 7.0.0` currently resolves through `notify-types 1.0.1`.
  - Remediation requires a compatible `notify` or `notify-types` release that
    removes or replaces `instant`.
- Risk rationale:
  - The advisory is an unmaintained-warning advisory, not a reported
    vulnerability.
  - Usage is transitive through filesystem notification infrastructure.
- Next review:
  - Quarterly, or when `notify` / `notify-types` updates.

### RUSTSEC-2024-0436 — `paste` 1.0.15

- Crate: `paste`
- Reason: unmaintained crate advisory.
- Severity: low; no known vulnerability is identified by the advisory.
- Dependency chains observed by `cargo audit`:
  - Embedding/vector path:
    - `paste 1.0.15`
    - `tokenizers 0.22.2`
    - `maenifold-core`
    - `maenifold-cli`, `maenifold-gate`, `maenifold-pipeline-driver`
  - Tokenizer macro helper path:
    - `paste 1.0.15`
    - `macro_rules_attribute 0.2.2`
    - `tokenizers 0.22.2`
    - `maenifold-core`
  - Native GUI lockfile paths:
    - `paste 1.0.15`
    - `metal 0.31.0`
    - `wgpu-hal 24.0.4`
    - `wgpu-core 24.0.5` / `wgpu 24.0.5`
    - `egui-wgpu 0.31.1`
    - `eframe 0.31.1`
    - `maenifold-gate`
    - `paste 1.0.15`
    - `accesskit_windows 0.24.1`
    - `accesskit_winit 0.23.1`
    - `egui-winit 0.31.1`
    - `eframe 0.31.1`
- Upstream blockers:
  - `tokenizers 0.22.2` is the current workspace parent for embedding/vector
    usage; remediation requires a compatible `tokenizers` release that removes
    or replaces `paste` and its `macro_rules_attribute` path.
  - Native GUI lockfile paths require compatible `eframe` / `wgpu` /
    `accesskit` updates that remove their transitive `paste` dependency.
- Risk rationale:
  - The advisory is an unmaintained-warning advisory, not a reported
    vulnerability.
  - Tokenizer usage is limited to embedding/vector model loading and inference.
  - GUI lockfile paths are transitive through the gate desktop/tray stack.
- Next review:
  - Quarterly, or when `tokenizers`, `eframe`, `wgpu`, or `accesskit` updates.

## Feature-gating assessment

`tokenizers` is directly used only under `crates/maenifold-core/src/vector/` for
embedding tokenizer loading. However, the vector module is referenced by search,
sync, similar-concept lookup, repair semantic validation, database vec0 setup,
and the `EmbeddingModels` tool. Feature-gating `tokenizers` alone would require
gating the broader embedding/vector API surface and adding non-embedding
fallback behavior across those modules.

Decision: defer feature-gating as a future optimization rather than making a
larger cross-cutting change in this advisory-documentation task.

Future optimization:

- Introduce an `embeddings` feature in `maenifold-core`.
- Move `ort`, `tokenizers`, and `hf-hub` behind that feature.
- Provide explicit fallback errors for semantic search, sync embedding
  generation, similar-concept vector lookup, semantic repair validation, and
  `EmbeddingModels` actions when the feature is disabled.
- Decide whether CLI and gate default builds should keep `embeddings` enabled or
  opt out for smaller / lower-RustSec-warning builds.

## Current audit verification

Command run on 2026-04-25:

```bash
cargo audit || true
```

Result:

- Expected accepted warning: `RUSTSEC-2024-0384` (`instant`, unmaintained).
- Expected accepted warning: `RUSTSEC-2024-0436` (`paste`, unmaintained).
- Additional warning observed: `RUSTSEC-2024-0370` (`proc-macro-error`,
  unmaintained) through GTK3 macro crates in the native tray lockfile path.
- Additional warning observed: `RUSTSEC-2024-0429` (`glib`, unsound) through
  GTK3 native tray lockfile paths.

The two GTK3/native-tray warnings were not part of the narrowed low-risk
accepted scope for this task. They appear in the lockfile through Linux-only
GTK3 dependencies under `tray-icon` / `muda`; current-target `cargo tree -i`
does not print a dependency path for `glib 0.18.5` or
`proc-macro-error 1.0.4` on this macOS worktree. They should remain tracked with
the existing native tray dependency-risk follow-up rather than being accepted
under the low-risk unmaintained rationale above.
