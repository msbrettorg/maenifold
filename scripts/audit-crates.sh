#!/usr/bin/env bash
set -euo pipefail

# Rust constitution audit — enforces build-time rules on crate source files.
#
# Rules:
#   max-loc        .rs files must be ≤300 lines (constitution hard limit)
#   no-todo        No TODO/FIXME/HACK/XXX markers (file a gate task instead)
#   no-unwrap      No .unwrap() in non-test code (use ? or expect with context)
#
# Exemption: // audit-ok: rule-name  on the same line.
#
# Usage:
#   ./scripts/audit-crates.sh           # scan all crate source
#   ./scripts/audit-crates.sh --report  # print summary only (no exit code)
#
# Exit code 0 = clean, 1 = violations found.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATES_DIR="$REPO_ROOT/crates"
MAX_LOC=300
violations=0
report_mode=false

[[ "${1:-}" == "--report" ]] && report_mode=true

# --- max-loc ---
while IFS= read -r file; do
  loc=$(wc -l < "$file")
  if (( loc > MAX_LOC )); then
    rel="${file#"$REPO_ROOT/"}"
    echo "  $rel:$loc  max-loc  $loc lines exceeds $MAX_LOC limit — split this module."
    ((violations++))
  fi
done < <(find "$CRATES_DIR" -name '*.rs' -not -path '*/target/*')

# --- no-todo ---
while IFS=: read -r file lineno line; do
  if [[ "$line" != *"audit-ok: no-todo"* ]]; then
    rel="${file#"$REPO_ROOT/"}"
    echo "  $rel:$lineno  no-todo  TODO/FIXME/HACK — resolve or file a gate task."
    ((violations++))
  fi
done < <(grep -rn -E '\b(TODO|FIXME|HACK|XXX)\b' "$CRATES_DIR" --include='*.rs' 2>/dev/null || true)

# --- no-unwrap (skip test files and test modules) ---
while IFS=: read -r file lineno line; do
  # Skip test files entirely
  [[ "$file" == *_test*.rs || "$file" == *tests/*.rs || "$file" == *test_*.rs ]] && continue
  # Skip lines inside #[cfg(test)] — heuristic: if line contains .unwrap() and file has test in name
  [[ "$line" == *"audit-ok: no-unwrap"* ]] && continue
  # Skip lines that are clearly in test context
  [[ "$line" == *"#[test]"* || "$line" == *"#[cfg(test)]"* ]] && continue
  rel="${file#"$REPO_ROOT/"}"
  echo "  $rel:$lineno  no-unwrap  .unwrap() in non-test code — use ? or .expect(\"context\")."
  ((violations++))
done < <(grep -rn '\.unwrap()' "$CRATES_DIR" --include='*.rs' 2>/dev/null | grep -v '_tests\.' | grep -v '#\[test\]' || true)

# --- summary ---
if (( violations > 0 )); then
  echo ""
  echo "❌ Rust audit: $violations violation(s) found."
  echo "Exempt a line with: // audit-ok: rule-name"
  $report_mode || exit 1
else
  echo "✅ audit:crates passed."
fi
