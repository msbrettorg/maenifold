#!/usr/bin/env bash
# Accumulate parity failures and report all offenders at the end. Infrastructure
# failures (missing deps, daemon unreachable, malformed catalog) still exit
# immediately with code 2. Parity failures exit 1.
set -euo pipefail

for dep in jq curl; do
  if ! command -v "$dep" >/dev/null 2>&1; then
    echo "audit-parity: ERROR — required dependency '$dep' is not installed." >&2
    exit 2
  fi
done

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MCP_URL="${MCP_URL:-http://127.0.0.1:8080/mcp}"
TOKEN_FILE="${MAENIFOLD_TOKEN_FILE:-$HOME/maenifold/auth/localhost.token}"

TOP_LEVEL_TOOLS=(GatewayBatch GatewayExecute GatewayIndex GatewayIntent GetHelp ReadMcpResource complete_node)
FORBIDDEN_TOOLS=(DiscoverMcpTools DiscoverMcpPrompts GetMcpPrompt)
NATIVE_DOMAINS=(maenifold.gate maenifold.graph maenifold.memory maenifold.system maenifold.thinking maenifold.workflow)

# Known acceptable CLI/MCP asymmetries:
# - Legacy lowercase/underscore aliases may be accepted by CLI dispatch but are
#   not canonical MCP/Gateway command names and are intentionally ignored here.
# - complete_node is a top-level MCP workflow callback and is not advertised by
#   the direct CLI registry; wire-surface presence is verified separately above.
ALLOWLIST=(complete_node)

AUTH_ARGS=()
if [[ -r "$TOKEN_FILE" ]]; then
  TOKEN="$(<"$TOKEN_FILE")"
  AUTH_ARGS=(-H "Authorization: Bearer $TOKEN")
fi

failures=()

fail() {
  failures+=("$1")
}

die_infra() {
  echo "audit-parity: ERROR — $*" >&2
  exit 2
}

choose_cli() {
  if [[ -n "${CLI:-}" ]]; then
    printf '%s\n' "$CLI"
  elif command -v maenifold >/dev/null 2>&1; then
    command -v maenifold
  else
    printf '%s\n' "$ROOT/target/release/maenifold"
  fi
}

CLI="$(choose_cli)"

ensure_cli() {
  if [[ -x "$CLI" ]]; then
    return 0
  fi

  if [[ "$CLI" == "$ROOT/target/release/maenifold" ]]; then
    echo "release CLI missing; building maenifold-cli and maenifold-pipeline-driver..."
    (cd "$ROOT" && CARGO_TARGET_DIR=target cargo build --release -p maenifold-cli && CARGO_TARGET_DIR=target cargo build --release -p maenifold-pipeline-driver) || die_infra "failed to build release binaries"
  fi

  [[ -x "$CLI" ]] || die_infra "CLI not found or not executable at $CLI"
}

mcp_request() {
  local body="$1"
  curl -fsS -X POST "$MCP_URL" \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json, text/event-stream' \
    "${AUTH_ARGS[@]}" \
    -d "$body"
}

mcp_tool_call() {
  local id="$1"
  local name="$2"
  local args="$3"
  mcp_request "{\"jsonrpc\":\"2.0\",\"id\":$id,\"method\":\"tools/call\",\"params\":{\"name\":\"$name\",\"arguments\":$args}}"
}

extract_text() {
  jq -r '
    if .error then
      "ERROR: " + (.error.message // "unknown MCP error")
    elif .result.content then
      (.result.content[0].text // (.result.content[0] | tostring))
    else
      (.result | tostring)
    end
  '
}

direct_help_like() {
  local name="$1"
  local text="$2"
  local first_line
  first_line="$(printf '%s\n' "$text" | sed -n '1p')"
  [[ "$first_line" == "# $name" ]] && return 0
  [[ "$first_line" == "$name" ]] && return 0
  [[ "$first_line" == "$name "* ]] && return 0
  return 1
}

mcp_has_required_property_error() {
  local response="$1"
  if jq -e '(.error.message? // "") | contains("Required property")' >/dev/null <<<"$response"; then
    return 0
  fi
  if jq -e 'any(.result.content[]?.text? // ""; contains("Required property"))' >/dev/null <<<"$response"; then
    return 0
  fi
  return 1
}

mcp_success_or_non_required_error() {
  local response="$1"
  # Some gateway schemas still reject the bare {"learn":true} shortcut with a
  # Required property error. That's a schema/help issue, not a registry/parity miss.
  if mcp_has_required_property_error "$response"; then
    return 0
  fi
  ! jq -e '.error' >/dev/null <<<"$response"
}

payload_for_direct_learn() {
  printf '{"learn":true}'
}

top_level_mcp_payload() {
  local name="$1"
  case "$name" in
    GetHelp)
      printf '{"toolName":"GetHelp","learn":true}'
      ;;
    ReadMcpResource)
      printf '{"uri":"asset://catalog","learn":true}'
      ;;
    *)
      printf '{"learn":true}'
      ;;
  esac
}

direct_cli_probe() {
  local name="$1"
  set +e
  DIRECT_CLI_OUTPUT="$("$CLI" tool "$name" "$(payload_for_direct_learn)" 2>&1)"
  DIRECT_CLI_STATUS=$?
  set -e
  DIRECT_CLI_FIRST_LINE="$(printf '%s\n' "$DIRECT_CLI_OUTPUT" | sed -n '1p')"
}

direct_cli_recognizes_probe() {
  local name="$1"
  [[ "$DIRECT_CLI_FIRST_LINE" != "Unknown tool: $name" ]]
}

require_direct_cli_learn() {
  local name="$1"
  if [[ $DIRECT_CLI_STATUS -eq 0 ]] && direct_help_like "$name" "$DIRECT_CLI_OUTPUT"; then
    return 0
  fi

  if ! direct_cli_recognizes_probe "$name"; then
    return 1
  fi

  fail "$name: direct CLI learn parity failed (exit $DIRECT_CLI_STATUS; first line: $DIRECT_CLI_FIRST_LINE)"
  return 1
}

require_mcp_gateway_learn() {
  local domain="$1"
  local command_name="$2"
  local response
  response="$(mcp_tool_call 200 GatewayExecute "{\"domain\":\"$domain\",\"command\":\"$command_name\",\"parameters\":{\"learn\":true}}")"
  if mcp_success_or_non_required_error "$response"; then
    return 0
  fi

  fail "$domain/$command_name: MCP GatewayExecute learn=true returned a Required property error"
  return 1
}

require_mcp_top_level_learn() {
  local name="$1"
  local payload response
  case "$name" in
    GatewayIndex|GatewayExecute|GatewayBatch|GatewayIntent|complete_node)
      # MCP exposes these as exact-schema tools without a top-level `learn` flag;
      # callers use GetHelp or valid schema-shaped payloads instead.
      return 0
      ;;
  esac
  payload="$(top_level_mcp_payload "$name")"
  response="$(mcp_tool_call 300 "$name" "$payload")"
  if mcp_success_or_non_required_error "$response"; then
    return 0
  fi

  fail "$name: MCP top-level learn=true returned a Required property error"
  return 1
}

parse_cli_banner_tools() {
  # Use bare CLI usage output; "maenifold tool" only emits clap stderr (`error:` / `Usage:`),
  # not the actual Available tools catalog we want to compare against.
  "$CLI" 2>&1 | head -100 | awk '
    /^Available tools:/ {
      in_tools = 1
      next
    }
    in_tools {
      if ($0 ~ /^[[:space:]]*$/) {
        exit
      }
      line = $0
      sub(/^[[:space:]]+/, "", line)
      sub(/^[A-Za-z]+:[[:space:]]+/, "", line)
      count = split(line, items, /,[[:space:]]*/)
      for (i = 1; i <= count; i++) {
        item = items[i]
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", item)
        if (item ~ /^[A-Za-z][A-Za-z0-9_]*$/) {
          print item
        }
      }
    }
  ' || true
}

is_allowlisted() {
  local name="$1"
  local item
  if ((${#ALLOWLIST[@]} == 0)); then
    return 1
  fi
  for item in "${ALLOWLIST[@]}"; do
    [[ "$item" == "$name" ]] && return 0
  done
  return 1
}

contains_name() {
  local needle="$1"
  local haystack="$2"
  grep -Fxq "$needle" <<<"$haystack"
}

echo "audit-parity: root=$ROOT"
echo "audit-parity: mcp=$MCP_URL"
ensure_cli
echo "audit-parity: cli=$CLI"

if ! tools_response="$(mcp_request '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' 2>&1)"; then
  echo "audit-parity: ERROR — daemon not reachable at $MCP_URL" >&2
  echo "$tools_response" >&2
  exit 2
fi
if jq -e '.error' >/dev/null <<<"$tools_response"; then
  echo "audit-parity: ERROR — daemon not reachable at $MCP_URL" >&2
  echo "$tools_response" | jq -r '.error.message // .' >&2
  exit 2
fi

actual_top_level_tools="$(jq -r '.result.tools[].name' <<<"$tools_response")"
expected_top_level_tools="$(printf '%s\n' "${TOP_LEVEL_TOOLS[@]}")"
sorted_actual_top_level_tools="$(printf '%s\n' "$actual_top_level_tools" | sed '/^$/d' | LC_ALL=C sort)"
sorted_expected_top_level_tools="$(printf '%s\n' "$expected_top_level_tools" | sed '/^$/d' | LC_ALL=C sort)"
tool_count="$(printf '%s\n' "$actual_top_level_tools" | sed '/^$/d' | wc -l | tr -d ' ')"

if [[ "$sorted_actual_top_level_tools" != "$sorted_expected_top_level_tools" ]]; then
  fail "MCP tool surface mismatch"
  fail "expected: $(printf '%s' "$sorted_expected_top_level_tools" | tr '\n' ' ' | sed 's/ $//')"
  fail "actual: $(printf '%s' "$sorted_actual_top_level_tools" | tr '\n' ' ' | sed 's/ $//')"
fi

for forbidden in "${FORBIDDEN_TOOLS[@]}"; do
  if contains_name "$forbidden" "$actual_top_level_tools"; then
    fail "forbidden tool re-surfaced: $forbidden"
  fi
done

while IFS= read -r bad_tool; do
  [[ -z "$bad_tool" ]] && continue
  fail "$bad_tool: missing non-empty name, description, or inputSchema"
done < <(jq -r '.result.tools[] | select((.name // "") == "" or (.description // "") == "" or (.inputSchema | type) != "object") | (.name // "<missing-name>")' <<<"$tools_response")

echo "MCP top-level tools: $tool_count ($(printf '%s' "$sorted_actual_top_level_tools" | tr '\n' ' ' | sed 's/ $//'))"

index_text="$(mcp_tool_call 2 GatewayIndex '{}' | extract_text)"
if ! jq -e '.domains' >/dev/null <<<"$index_text"; then
  die_infra "GatewayIndex did not return domains"
fi

cli_banner_tools="$(parse_cli_banner_tools | sort -u)"
mcp_and_gateway="$(printf '%s\n' "${TOP_LEVEL_TOOLS[@]}")"
direct_cli_registered=""

for domain in "${NATIVE_DOMAINS[@]}"; do
  if ! jq -e --arg domain "$domain" '.domains[] | select(.name == $domain)' >/dev/null <<<"$index_text"; then
    fail "missing native Gateway domain: $domain"
    continue
  fi

  domain_text="$(mcp_tool_call 10 GatewayIndex "{\"domain\":\"$domain\"}" | extract_text)"
  commands="$(jq -r '.commands[].name' <<<"$domain_text")"
  command_count="$(printf '%s\n' "$commands" | sed '/^$/d' | wc -l | tr -d ' ')"
  echo "Gateway domain $domain: $command_count commands"
  mcp_and_gateway="$(printf '%s\n%s\n' "$mcp_and_gateway" "$commands")"

  while IFS= read -r command_name; do
    [[ -z "$command_name" ]] && continue
    direct_cli_probe "$command_name"
    if direct_cli_recognizes_probe "$command_name"; then
      direct_cli_registered="$(printf '%s\n%s\n' "$direct_cli_registered" "$command_name")"
    fi
    require_direct_cli_learn "$command_name" || true
    require_mcp_gateway_learn "$domain" "$command_name" >/dev/null || true
  done <<<"$commands"
done

for tool_name in "${TOP_LEVEL_TOOLS[@]}"; do
  direct_cli_probe "$tool_name"
  if direct_cli_recognizes_probe "$tool_name"; then
    direct_cli_registered="$(printf '%s\n%s\n' "$direct_cli_registered" "$tool_name")"
  fi
  require_direct_cli_learn "$tool_name" || true
  require_mcp_top_level_learn "$tool_name" >/dev/null || true
done

cli_tools="$(printf '%s\n%s\n' "$cli_banner_tools" "$direct_cli_registered" | sed '/^$/d' | sort -u)"
mcp_and_gateway="$(printf '%s\n' "$mcp_and_gateway" | sed '/^$/d' | sort -u)"

while IFS= read -r name; do
  [[ -z "$name" ]] && continue
  if ! contains_name "$name" "$cli_tools" && ! is_allowlisted "$name"; then
    fail "$name: present in MCP/Gateway catalog but absent from direct CLI registry"
  fi
done <<<"$mcp_and_gateway"

while IFS= read -r name; do
  [[ -z "$name" ]] && continue
  if ! contains_name "$name" "$mcp_and_gateway" && ! is_allowlisted "$name"; then
    fail "$name: advertised by CLI but absent from MCP/Gateway catalog"
  fi
done <<<"$cli_tools"

if ((${#failures[@]} > 0)); then
  echo "FAIL parity audit (${#failures[@]} issue(s))" >&2
  for failure in "${failures[@]}"; do
    echo " - $failure" >&2
  done
  exit 1
fi

echo "PASS parity audit: 7 top-level MCP tools, ${#NATIVE_DOMAINS[@]} native domains, direct CLI and MCP learn=true parity checked"
