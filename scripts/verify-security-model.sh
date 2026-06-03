#!/bin/bash
set -euo pipefail

DOC="${MAENIFOLD_SKILL_DIR:-$HOME/.copilot/skills/maenifold}/references/SECURITY_MODEL.md"
checks_passed=0

fail() {
  echo "$1" >&2
  exit 1
}

check_section() {
  local heading="$1"
  local line="## $heading"
  local body_lines

  grep -Fxq "$line" "$DOC" || fail "missing section: $heading"

  body_lines=$(
    awk -v heading="$line" '
      $0 == heading { in_section=1; next }
      in_section && /^## / { exit }
      in_section && /[^[:space:]]/ { count++ }
      END { print count + 0 }
    ' "$DOC"
  )

  if [ "$body_lines" -lt 5 ]; then
    fail "section too short: $heading ($body_lines non-empty lines)"
  fi

  checks_passed=$((checks_passed + 1))
}

check_contains() {
  local description="$1"
  local literal="$2"

  grep -Fq "$literal" "$DOC" || fail "missing literal: $literal ($description)"
  checks_passed=$((checks_passed + 1))
}

check_does_not_contain() {
  local description="$1"
  local literal="$2"

  if grep -Fq "$literal" "$DOC"; then
    fail "forbidden literal present: $literal ($description)"
  fi
  checks_passed=$((checks_passed + 1))
}

check_contains_within() {
  local description="$1"
  local literal_a="$2"
  local literal_b="$3"
  local within="$4"

  awk -v a="$literal_a" -v b="$literal_b" -v w="$within" -v description="$description" '
    BEGIN {
      in_section = 0
      body = ""
    }
    $0 == "## DNS rebinding" {
      in_section = 1
      next
    }
    in_section && /^## / {
      in_section = 0
    }
    in_section {
      body = body $0 "\n"
    }
    END {
      pa = index(body, a)
      pb = index(body, b)
      if (pa > 0 && pb > 0) {
        d = pa - pb
        if (d < 0) {
          d = -d
        }
        if (d <= w) {
          exit 0
        }
      }
      printf "literals not within %s chars of each other in DNS rebinding section (%s)\n", w, description > "/dev/stderr"
      exit 1
    }
  ' "$DOC" || fail "literals not within $within chars of each other in DNS rebinding section ($description)"

  checks_passed=$((checks_passed + 1))
}

if [ ! -f "$DOC" ]; then
  fail "missing file: $DOC"
fi

check_section "Listener topology"
check_section "Trust model"
check_section "DNS rebinding"
check_section "What auth is NOT in scope"
check_section "Operator obligations when exposing to LAN"

check_contains "MCP listener default" "127.0.0.1:8080"
check_contains "local API default" "127.0.0.1:8081"
check_contains "allowed hosts config key" "mcpAllowedHosts"
check_contains "allowed origins config key" "mcpAllowedOrigins"
check_contains "rmcp allowed hosts builder" "with_allowed_hosts"
check_contains "rmcp allowed origins builder" "with_allowed_origins"
check_contains "explicit rmcp builder REPLACES semantics" "REPLACES"
check_contains_within "origin builder REPLACES semantics co-located" "with_allowed_origins" "REPLACES" 1000
check_contains_within "host builder REPLACES semantics co-located" "with_allowed_hosts" "REPLACES" 1000
check_contains "operator obligation" "Tailnet Lock"
check_contains "operator obligation" "FIDO2"
check_contains "MCP source cross-reference" "crates/maenifold-core/src/mcp/http.rs"
check_contains "ACP source cross-reference" "crates/maenifold-core/src/api/acp_relay.rs"
check_contains "opaque-origin lesson" "Origin: null"
check_contains "opaque-origin lesson" "Origin: file://"
check_does_not_contain "claim that loopback enforces UID isolation" "UID isolation"
check_does_not_contain "claim that loopback enforces UID isolation (variant)" "UID-isolated"
check_does_not_contain "claim that loopback enforces UID isolation (variant)" "UID isolated"
check_does_not_contain "ambiguous extends-or-replaces wording" "extends or replaces"
check_does_not_contain "ambiguous extend wording" "extends the default"

echo "✓ $checks_passed checks passed"
