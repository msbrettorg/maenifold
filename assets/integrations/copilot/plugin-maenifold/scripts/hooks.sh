#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MCP_URL="http://127.0.0.1:8080/mcp"
TOKEN_FILE="$HOME/maenifold/auth/localhost.token"

source "$SCRIPT_DIR/lib/ingest_checkpoints.sh"

usage() {
  echo '{"error":"Usage: hooks.sh {session_start|subagent_prime|pre_compact|subagent_stop|agent_stop|tool_failure}"}' >&2
  exit 1
}

read_hook_input() {
  cat
}

extract_response_body() {
  jq -r '
    if .result.contents?[0].text? then .result.contents[0].text
    elif .result.content?[0].text? then .result.content[0].text
    elif .result.text? then .result.text
    elif .result? then (.result | tojson)
    else empty end
  ' 2>/dev/null
}

extract_primer_lines() {
  jq -er '
    select(type == "object" and (.lines? | type == "array") and (.lines | length > 0) and all(.lines[]; type == "string"))
    | .lines
    | join("\n")
  ' 2>/dev/null
}

format_graph_summary() {
  jq -er '
    if type == "object" and (.nodes? | type == "array") then
      "snapshot nodes=\(.nodes | length) edges=\(.edges | length) generated_at=\(.generated_at // "unknown")"
    elif type == "object" and .node_count? != null then
      "stats nodes=\(.node_count) edges=\(.edge_count) communities=\(.community_count)"
    else
      empty
    end
  ' 2>/dev/null
}

read_token() {
  [[ -f "$TOKEN_FILE" ]] || return 1
  local token
  token="$(tr -d '\r\n' < "$TOKEN_FILE" 2>/dev/null)"
  [[ -n "$token" ]] || return 1
  printf '%s' "$token"
}

call_daemon() {
  local payload="$1"
  local timeout="${2:-5}"
  local token
  token="$(read_token)" || return 1
  curl -sS --max-time "$timeout" -X POST "$MCP_URL" \
    -H 'Content-Type: application/json' \
    -H 'Accept: application/json, text/event-stream' \
    -H 'Mcp-Protocol-Version: 2025-03-26' \
    -H "Authorization: Bearer $token" \
    -d "$payload" 2>/dev/null
}

mcp_read_resource() {
  local uri="$1"
  local response
  response="$(call_daemon "$(jq -nc --arg uri "$uri" '{jsonrpc:"2.0",id:1,method:"resources/read",params:{uri:$uri}}')" 5)" || return 1
  printf '%s' "$response" | extract_response_body
}

mcp_call_tool() {
  local name="$1"
  local args_json="${2:-}"
  local timeout="${3:-5}"
  local response
  [[ -n "$args_json" ]] || args_json='{}'
  response="$(call_daemon "$(jq -nc --arg name "$name" --argjson arguments "$args_json" '{jsonrpc:"2.0",id:1,method:"tools/call",params:{name:$name,arguments:$arguments}}')" "$timeout")" || return 1
  printf '%s' "$response" | extract_response_body
}

extract_wikilinks() {
  grep -oE '\[\[[^][]+\]\]' | sed 's/^\[\[//; s/\]\]$//' | tr '[:upper:]' '[:lower:]' | tr ' ' '-' | awk 'NF && !seen[$0]++'
}

last_transcript_message() {
  local transcript_path="$1"
  local role="$2"
  [[ -f "$transcript_path" ]] || return 1
  jq -sr --arg role "$role" '[.[] | select(.role == $role) | .content][-1] // empty' "$transcript_path" 2>/dev/null
}

is_substantial_response() {
  local content="${1:-}"
  [[ ${#content} -ge 120 ]]
}

has_markdown_section() {
  local content="$1"
  local name="$2"
  printf '%s\n' "$content" | grep -Eq "^#{1,6}[[:space:]]+${name}[[:space:]]*$"
}

emit_context() {
  local context="${1:-}"
  [[ -n "$context" ]] || return 0
  jq -nc --arg additionalContext "$context" '{additionalContext:$additionalContext}'
}

maybe_emit_block() {
  local transcript_path="$1"
  local assistant_content
  assistant_content="$(last_transcript_message "$transcript_path" assistant || true)"
  [[ -n "$assistant_content" ]] || return 0
  is_substantial_response "$assistant_content" || return 0

  local has_links="false"
  local has_confession="false"
  local has_confession_sections="false"
  [[ "$assistant_content" =~ \[\[[^][]+\]\] ]] && has_links="true"
  [[ "$assistant_content" == *"ConfessionReport"* ]] && has_confession="true"
  if has_markdown_section "$assistant_content" "Compliance" &&
    has_markdown_section "$assistant_content" "Files" &&
    has_markdown_section "$assistant_content" "Gaps" &&
    has_markdown_section "$assistant_content" "WikiLinks"; then
    has_confession_sections="true"
  fi
  [[ "$has_links" == "true" && "$has_confession" == "true" && "$has_confession_sections" == "true" ]] && return 0

  local reason="Your last substantial response must include durable [[WikiLinks]] and a ConfessionReport with Compliance, Files, Gaps, and WikiLinks sections."
  if [[ "$has_confession" == "true" && "$has_confession_sections" == "false" ]]; then
    reason="Your last substantial response included ConfessionReport, but it must contain Compliance, Files, Gaps, and WikiLinks sections."
  elif [[ "$has_confession" == "false" && "$has_confession_sections" == "true" && "$has_links" == "true" ]]; then
    reason="Your last substantial response includes the four required sections and durable [[WikiLinks]], but it is missing the ConfessionReport label. Add a 'ConfessionReport' marker heading to the report."
  elif [[ "$has_links" == "false" && "$has_confession_sections" == "true" ]]; then
    reason="Your last substantial response is missing durable [[WikiLinks]]. Tag important concepts, decisions, and entities; do not tag trivial words."
  elif [[ "$has_links" == "true" && "$has_confession" == "false" ]]; then
    reason="Your last substantial response is missing a ConfessionReport with Compliance, Files, Gaps, and WikiLinks sections."
  fi
  jq -nc --arg reason "$reason" '{decision:"block",reason:$reason}'
}

known_tool() {
  local tool_name="$1"
  if command -v maenifold >/dev/null 2>&1; then
    maenifold tool list 2>/dev/null | grep -qx "$tool_name" && return 0
  fi
  case "$tool_name" in
    AnalyzeConceptCorruption|AcpExec|Adopt|BuildContext|DeleteMemory|EditMemory|ExtractConceptsFromFile|FindSimilarConcepts|GetConfig|GetHelp|ListAssets|ListMemories|ListSessions|MemoryStatus|ReadMemory|RecentActivity|RepairConcepts|RunFullBenchmark|Schedule|SearchMemories|Sync|TeamsMonitor|UpdateAssets|Visualize|WriteMemory) return 0 ;;
    *) return 1 ;;
  esac
}

session_start() {
  local input session_id graph_summary recent_activity context
  input="$(read_hook_input)"
  session_id="$(printf '%s' "$input" | jq -r '.sessionId // empty' 2>/dev/null)"
  [[ -n "$session_id" ]] && ingest_checkpoints "$session_id" || true

  if ! graph_summary="$(mcp_read_resource 'graph://corpus/primer' | extract_primer_lines)"; then
    graph_summary="$(mcp_read_resource 'graph://corpus/snapshot' || true)"
    if ! graph_summary="$(printf '%s' "$graph_summary" | format_graph_summary)"; then
      graph_summary="$(mcp_read_resource 'graph://stats' || true)"
      graph_summary="$(printf '%s' "$graph_summary" | format_graph_summary || true)"
    fi
  fi

  recent_activity="$(mcp_call_tool 'RecentActivity' '{"limit":5}' 5 || true)"
  if printf '%s' "$recent_activity" | jq -e 'type == "object" and .files? | type == "array"' >/dev/null 2>&1; then
    recent_activity="$(printf '%s' "$recent_activity" | jq -r '.files[:5] | join(", ")')"
  fi

  [[ -n "$graph_summary$recent_activity" ]] || return 0
  context=$'# maenifold\n\nTag important concepts as [[WikiLinks]]. Use lowercase-hyphenated concepts and skip trivial words.\n'
  [[ -n "$graph_summary" ]] && context+=$'\n<communities>\n'"$graph_summary"$'\n</communities>\n'
  [[ -n "$recent_activity" ]] && context+=$'\n<recent-activity>\n'"$recent_activity"$'\n</recent-activity>\n'
  emit_context "$context"
}

subagent_prime() {
  local input transcript_path task_prompt concepts community_ids community_id community_text context
  input="$(read_hook_input)"
  transcript_path="$(printf '%s' "$input" | jq -r '.transcriptPath // empty' 2>/dev/null)"
  [[ -n "$transcript_path" && -f "$transcript_path" ]] || return 0

  task_prompt="$(last_transcript_message "$transcript_path" user || true)"
  [[ -n "$task_prompt" ]] || return 0
  concepts="$(printf '%s' "$task_prompt" | extract_wikilinks | head -n 10)"
  [[ -n "$concepts" ]] || return 0

  community_ids="$(while IFS= read -r concept; do
    local concept_json id
    [[ -n "$concept" ]] || continue
    concept_json="$(mcp_read_resource "graph://concept/$concept" || true)"
    id="$(printf '%s' "$concept_json" | jq -r '.communityId // empty' 2>/dev/null)"
    [[ -n "$id" ]] && printf '%s\n' "$id"
  done <<< "$concepts" | awk '!seen[$0]++' | head -n 5)"
  [[ -n "$community_ids" ]] || return 0

  context=$'# maenifold community context\n'
  while IFS= read -r community_id; do
    [[ -n "$community_id" ]] || continue
    community_text="$(mcp_read_resource "graph://community/$community_id" || true)"
    if printf '%s' "$community_text" | jq -e 'type == "object" and .text? != null' >/dev/null 2>&1; then
      community_text="$(printf '%s' "$community_text" | jq -r '.text')"
    fi
    [[ -n "$community_text" ]] && context+=$'\n- '"$community_text"
  done <<< "$community_ids"
  emit_context "$context"
}

pre_compact() {
  local input session_id
  input="$(read_hook_input)"
  session_id="$(printf '%s' "$input" | jq -r '.sessionId // empty' 2>/dev/null)"
  [[ -n "$session_id" ]] && ingest_checkpoints "$session_id" || true
  echo '{}' # Notification-only output stays empty-ish; this hook currently just preserves checkpoints.
}

subagent_stop() {
  local input session_id transcript_path
  input="$(read_hook_input)"
  session_id="$(printf '%s' "$input" | jq -r '.sessionId // empty' 2>/dev/null)"
  transcript_path="$(printf '%s' "$input" | jq -r '.transcriptPath // empty' 2>/dev/null)"
  [[ -n "$session_id" ]] && ingest_checkpoints "$session_id" || true
  [[ -n "$transcript_path" ]] || return 0
  maybe_emit_block "$transcript_path"
}

agent_stop() {
  local input session_id transcript_path
  input="$(read_hook_input)"
  session_id="$(printf '%s' "$input" | jq -r '.sessionId // empty' 2>/dev/null)"
  transcript_path="$(printf '%s' "$input" | jq -r '.transcriptPath // empty' 2>/dev/null)"
  [[ -n "$session_id" ]] && ingest_checkpoints "$session_id" || true
  [[ -n "$transcript_path" ]] || return 0
  maybe_emit_block "$transcript_path"
}

tool_failure() {
  local input tool_name help_text
  input="$(read_hook_input)"
  tool_name="$(printf '%s' "$input" | jq -r '.toolName // empty' 2>/dev/null)"
  [[ -n "$tool_name" ]] || return 0
  known_tool "$tool_name" || return 0
  help_text="$(mcp_call_tool "$tool_name" '{"learn":true}' 5 || true)"
  [[ -n "$help_text" ]] || return 0
  emit_context "$help_text"
}

[[ -n "$MODE" ]] || usage
case "$MODE" in
  session_start) session_start ;;
  subagent_prime) subagent_prime ;;
  pre_compact) pre_compact ;;
  subagent_stop) subagent_stop ;;
  agent_stop) agent_stop ;;
  tool_failure) tool_failure ;;
  *) usage ;;
esac
