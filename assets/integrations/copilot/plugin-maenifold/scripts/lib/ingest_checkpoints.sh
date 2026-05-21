#!/usr/bin/env bash

checkpoint_exists() {
  local title="$1"
  local response
  response="$(mcp_call_tool 'SearchMemories' "$(jq -nc --arg query "$title" '{query:$query,pageSize:5}')" 5 || true)"
  [[ -n "$response" ]] || return 1
  printf '%s' "$response" | jq -e --arg title "$title" '
    [.results[]? | .payload? | fromjson? | .params?.arguments?.title?]
    | index($title) != null
  ' >/dev/null 2>&1
}

ingest_checkpoints() {
  local session_id="$1"
  local checkpoint_dir="${COPILOT_SESSION_STATE:-}/$session_id/checkpoints"
  [[ -n "${COPILOT_SESSION_STATE:-}" && -d "$checkpoint_dir" ]] || return 0

  local today folder file content slug title
  today="$(date -u +%Y/%m/%d)"
  folder="thinking/compaction/$today"

  for file in "$checkpoint_dir"/*.md; do
    [[ -f "$file" ]] || continue
    [[ "$(basename "$file")" == "index.md" ]] && continue
    content="$(cat "$file" 2>/dev/null || true)"
    [[ "$content" =~ \[\[[^][]+\]\] ]] || continue
    slug="$(basename "$file" .md)"
    title="checkpoint-$session_id-$slug"
    checkpoint_exists "$title" && continue
    mcp_call_tool 'WriteMemory' "$(jq -nc --arg title "$title" --arg folder "$folder" --arg content "$content" '{title:$title,folder:$folder,content:$content}')" 5 >/dev/null || true
  done
}
