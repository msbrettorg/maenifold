#!/bin/bash

set -u

MODEL="claude-opus-4.6"
MAX_ITERATIONS=50
CRATE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --model)
      MODEL="${2:-}"
      shift 2
      ;;
    --max-iterations)
      MAX_ITERATIONS="${2:-}"
      shift 2
      ;;
    --crate)
      CRATE="${2:-}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1"
      echo "Usage: $0 [--model <model>] [--max-iterations <n>] [--crate <crate>]"
      exit 1
      ;;
  esac
done

mkdir -p ralph/state

for ((iteration=1; iteration<=MAX_ITERATIONS; iteration++)); do
  if [[ -f "ralph/state/converged" ]]; then
    echo "Converged marker found before iteration ${iteration}; exiting loop."
    break
  fi

  start_epoch="$(date +%s)"
  echo "=== RALPH iteration ${iteration}/${MAX_ITERATIONS} (model=${MODEL}, crate=${CRATE:-all}) ==="

  RALPH_TARGET_CRATE="$CRATE" copilot -p "$(cat ralph/prompt.md)" --agent product-manager --yolo --model "$MODEL"
  exit_code=$?

  end_epoch="$(date +%s)"
  duration=$((end_epoch - start_epoch))
  echo "Iteration ${iteration} complete: duration=${duration}s exit_code=${exit_code}"

  if [[ -f "ralph/state/converged" ]]; then
    echo "Converged marker found after iteration ${iteration}; exiting loop."
    break
  fi
done
