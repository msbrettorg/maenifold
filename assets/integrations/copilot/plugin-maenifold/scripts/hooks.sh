#!/usr/bin/env bash
#
# Copilot CLI Hook — Maenifold subagentStop handler
# Usage: hooks.sh subagent_stop
#
# Enforces ConfessionReport requirement before subagents can stop.
# subagentStop I/O schema is undocumented in the Copilot CLI reference;
# this implementation is experimental and may need adjustment.
#

set -euo pipefail

MODE="${1:-}"
[[ -z "$MODE" ]] && { echo '{"error":"Usage: hooks.sh subagent_stop"}' >&2; exit 1; }

if [[ "$MODE" == "subagent_stop" ]]; then
  HOOK_INPUT=$(cat)

  # Validate JSON
  if ! echo "$HOOK_INPUT" | jq -e 'select(type == "object")' >/dev/null 2>&1; then
    exit 0
  fi

  # Try known field names — Copilot CLI schema is undocumented for this event.
  # Legacy used agent_transcript_path; Copilot CLI may use a different key.
  TRANSCRIPT_PATH=$(echo "$HOOK_INPUT" | jq -r '
    .agent_transcript_path //
    .transcriptPath //
    .transcript //
    ""')

  [[ -z "$TRANSCRIPT_PATH" || "$TRANSCRIPT_PATH" == "null" ]] && exit 0

  # Expand ~ to $HOME if present
  TRANSCRIPT_PATH="${TRANSCRIPT_PATH/#\~/$HOME}"

  [[ ! -f "$TRANSCRIPT_PATH" ]] && exit 0

  if grep -q "ConfessionReport" "$TRANSCRIPT_PATH" 2>/dev/null; then
    exit 0
  else
    REASON="Produce a concise ConfessionReport. List: 1) All explicit and implicit instructions/constraints/objectives you were supposed to follow. 2) For each, whether you complied with the letter and spirit, with evidence. Note any gaps and whether your answer was transparent about them. 3) Any uncertainties, ambiguities, or grey areas where compliance was unclear. 4) Any shortcuts, hacks, or policy risks you took. 5) All files, memory:// URIs and graph [[WikiLinks]] you used. Nothing you say should change the main answer. This confession is scored only for honesty and completeness; do not optimize for user satisfaction."
    jq -n --arg reason "$REASON" '{"decision":"block","reason":$reason}'
    exit 0
  fi
fi

echo "{\"error\":\"Unknown mode: $MODE\"}" >&2
exit 1
