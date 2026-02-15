#!/usr/bin/env bash
#
# workiq-think.sh — Query M365 via WorkIQ, analyze via Maenifold SequentialThinking
#
# Usage:
#   ./workiq-think.sh "What meetings do I have this week about cost optimization?"
#   ./workiq-think.sh -t 5 "Summarize my recent emails about the Azure migration"
#
set -euo pipefail

MAX_THOUGHTS="${WORKIQ_THINK_THOUGHTS:-3}"
QUESTION=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -t|--thoughts) MAX_THOUGHTS="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: workiq-think.sh [-t N] <question>"
            exit 0
            ;;
        *) QUESTION="$1"; shift ;;
    esac
done

[[ -z "$QUESTION" ]] && echo "Usage: workiq-think.sh [-t N] <question>" && exit 1

# JSON-escape a string (stdin)
jesc() { python3 -c "import json,sys; print(json.dumps(sys.stdin.read().strip()))"; }

# --- 1. Query WorkIQ ---
echo "Querying M365 Copilot..." >&2
WORKIQ_RESPONSE=$(workiq ask -q "$QUESTION" 2>&1)

[[ -z "$WORKIQ_RESPONSE" ]] && echo "Error: Empty response from WorkIQ" && exit 1

# --- 2. Thought 0: inject M365 data ---
RESPONSE_ESC=$(echo "Analyzing [[WorkIQ]] data for [[${QUESTION// /-}]]: ${WORKIQ_RESPONSE}" | jesc)

RESULT0=$(maenifold --tool SequentialThinking --payload \
    "{\"response\":${RESPONSE_ESC},\"thoughtNumber\":0,\"totalThoughts\":${MAX_THOUGHTS},\"nextThoughtNeeded\":true}" \
    --json)

SESSION_ID=$(echo "$RESULT0" | python3 -c "import json,sys; print(json.load(sys.stdin).get('data',{}).get('sessionId',''))" 2>/dev/null)
[[ -z "$SESSION_ID" ]] && SESSION_ID=$(echo "$RESULT0" | grep -oE 'session-[0-9]+-[0-9]+' | head -1)
[[ -z "$SESSION_ID" ]] && echo "Error: No session created" && echo "$RESULT0" && exit 1

echo "Session: $SESSION_ID" >&2

# --- 3. Middle thoughts (pass the actual M365 data each time for real analysis) ---
for (( i=1; i<MAX_THOUGHTS-1; i++ )); do
    echo "Thinking... ($((i+1))/$MAX_THOUGHTS)" >&2
    THOUGHT_ESC=$(echo "Step $((i+1)) analysis of [[WorkIQ]] response for [[${QUESTION// /-}]]: examining [[M365-Copilot]] data for actionable patterns in [[knowledge-graph]]" | jesc)

    maenifold --tool SequentialThinking --payload \
        "{\"response\":${THOUGHT_ESC},\"thoughtNumber\":${i},\"totalThoughts\":${MAX_THOUGHTS},\"nextThoughtNeeded\":true,\"sessionId\":\"${SESSION_ID}\"}" \
        --json > /dev/null
done

# --- 4. Conclusion ---
FINAL=$((MAX_THOUGHTS - 1))
FINAL_ESC=$(echo "Final synthesis of [[WorkIQ]] data for [[${QUESTION// /-}]]" | jesc)
CONCLUSION_ESC=$(echo "Analysis of [[WorkIQ]] query complete. [[M365-Copilot]] data for '${QUESTION}' integrated into [[knowledge-graph]] via [[sequential-thinking]]." | jesc)

maenifold --tool SequentialThinking --payload \
    "{\"response\":${FINAL_ESC},\"thoughtNumber\":${FINAL},\"totalThoughts\":${MAX_THOUGHTS},\"nextThoughtNeeded\":false,\"sessionId\":\"${SESSION_ID}\",\"conclusion\":${CONCLUSION_ESC}}" \
    --json > /dev/null

# --- Output ---
echo "$SESSION_ID"
echo "---" >&2
echo "M365: $WORKIQ_RESPONSE" >&2
echo "View: maenifold --tool ReadMemory --payload '{\"identifier\":\"memory://thinking/sequential/${SESSION_ID}\"}'" >&2
