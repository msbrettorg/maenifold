#!/bin/bash
# Reject any commit that modifies BOTH ralph/prompt.md (or ralph.sh) AND product code.
# Run this in CI on every PR / push.
set -euo pipefail
BASE=${1:-$(git merge-base HEAD dev)}
HEAD=${2:-HEAD}
FILES=$(git diff --name-only "$BASE..$HEAD")
echo "$FILES" | grep -qE "^(ralph/prompt\.md|ralph\.sh)$" || { echo "OK: no ralph harness changes"; exit 0; }
PRODUCT=$(echo "$FILES" | grep -vE "^(ralph/|ralph\.sh|done/|todo/|docs/|README|\.gitignore|\.gate/)" || true)
if [ -n "$PRODUCT" ]; then
    echo "ERROR: this change set modifies BOTH ralph harness AND product code:"
    echo "  ralph files:"
    echo "$FILES" | grep -E "^(ralph/prompt\.md|ralph\.sh)$" | sed 's/^/    /'
    echo "  product files:"
    echo "$PRODUCT" | sed 's/^/    /'
    echo "Split into separate commits or escalate to PM for explicit override."
    exit 1
fi
echo "OK: ralph-only change set (no product code)"
