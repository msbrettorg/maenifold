#!/usr/bin/env bash
set -euo pipefail

# gpt-image-2 — Azure AI Foundry image generator
# Usage:
#   generate.sh "your prompt here"                        # defaults: 1536x1024, medium, png
#   generate.sh "your prompt" --size 1024x1024            # custom size
#   generate.sh "your prompt" --quality high              # low|medium|high
#   generate.sh "your prompt" --size 3840x2160 --quality high --label hero  # 4K
#   generate.sh "your prompt" --n 4                       # multiple variants
#   generate.sh "your prompt" --transparent               # transparent background
#   generate.sh "your prompt" --open                      # open in Preview after saving
#   generate.sh @prompt.txt                               # read prompt from file
#   generate.sh @prompt.txt --size 3840x2160 --quality high --timeout 600
#
# 4K + high quality can take 2-5 minutes. The script auto-retries on 429 rate limits.
# Default timeout is 300s (5 min). For 4K high, use --timeout 600.
#
# Environment:
#   AZURE_OPENAI_API_KEY      — required (auto-loaded from ~/.secrets/azure_openai_api_key)
#   AZURE_OPENAI_ENDPOINT     — required (default: https://foundy1125.openai.azure.com)
#   GPT_IMAGE_DEPLOYMENT      — optional (default: gpt-image-2)
#   GPT_IMAGE_API_VERSION     — optional (default: 2025-04-01-preview)
#   GPT_IMAGE_OUTPUT_DIR      — optional (default: ./output)

# ── Load API key ──────────────────────────────────────────────────────────────
if [ -z "${AZURE_OPENAI_API_KEY:-}" ]; then
  if [ -r "$HOME/.secrets/azure_openai_api_key" ]; then
    export AZURE_OPENAI_API_KEY="$(cat "$HOME/.secrets/azure_openai_api_key")"
  else
    echo "ERROR: AZURE_OPENAI_API_KEY not set and ~/.secrets/azure_openai_api_key not found" >&2
    exit 1
  fi
fi

# ── Defaults ──────────────────────────────────────────────────────────────────
ENDPOINT="${AZURE_OPENAI_ENDPOINT:-https://foundy1125.openai.azure.com}"
DEPLOYMENT="${GPT_IMAGE_DEPLOYMENT:-gpt-image-2}"
API_VERSION="${GPT_IMAGE_API_VERSION:-2025-04-01-preview}"
OUTPUT_DIR="${GPT_IMAGE_OUTPUT_DIR:-./output}"
SIZE="1536x1024"
QUALITY="medium"
FORMAT="png"
N=1
LABEL="gpt2"
BACKGROUND="auto"
TIMEOUT=300
OPEN_AFTER=false
MAX_RETRIES=5
DRY_RUN=false

# ── Parse args ────────────────────────────────────────────────────────────────
PROMPT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --size)       SIZE="$2";       shift 2 ;;
    --quality)    QUALITY="$2";    shift 2 ;;
    --format)     FORMAT="$2";     shift 2 ;;
    --n)          N="$2";          shift 2 ;;
    --label)      LABEL="$2";      shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    --timeout)    TIMEOUT="$2";    shift 2 ;;
    --retries)    MAX_RETRIES="$2"; shift 2 ;;
    --dry-run)    DRY_RUN=true; shift ;;
    --transparent) BACKGROUND="transparent"; FORMAT="png"; shift ;;
    --open)       OPEN_AFTER=true; shift ;;
    --help|-h)
      sed -n '3,19p' "$0"
      exit 0 ;;
    @*)
      PROMPT_FILE="${1#@}"
      if [ ! -r "$PROMPT_FILE" ]; then
        echo "ERROR: Cannot read prompt file: $PROMPT_FILE" >&2
        exit 1
      fi
      PROMPT="$(cat "$PROMPT_FILE")"
      shift ;;
    *)
      if [ -z "$PROMPT" ]; then
        PROMPT="$1"
      else
        PROMPT="$PROMPT $1"
      fi
      shift ;;
  esac
done

if [ -z "$PROMPT" ]; then
  echo "Usage: generate.sh \"your prompt\" [--size WxH] [--quality low|medium|high] [--open]" >&2
  echo "       generate.sh @prompt.txt [--size 3840x2160] [--quality high] [--timeout 600]" >&2
  exit 1
fi

# ── Build request ─────────────────────────────────────────────────────────────
URL="${ENDPOINT}/openai/deployments/${DEPLOYMENT}/images/generations?api-version=${API_VERSION}"
mkdir -p "$OUTPUT_DIR"

BODY=$(python3 -c "
import json, sys
body = {
    'prompt': sys.stdin.read(),
    'n': $N,
    'size': '$SIZE',
    'quality': '$QUALITY',
    'output_format': '$FORMAT',
    'background': '$BACKGROUND'
}
print(json.dumps(body))
" <<< "$PROMPT")

# ── Dry run — preview only ────────────────────────────────────────────────────
if [ "$DRY_RUN" = true ]; then
  echo "── DRY RUN (no API call) ──"
  echo ""
  echo "Endpoint:  ${URL}"
  echo "Size:      ${SIZE}"
  echo "Quality:   ${QUALITY}"
  echo "Format:    ${FORMAT}"
  echo "N:         ${N}"
  echo "Background:${BACKGROUND}"
  echo "Timeout:   ${TIMEOUT}s"
  echo "Output:    ${OUTPUT_DIR}/${LABEL}_*.${FORMAT}"
  echo ""
  echo "── PROMPT ──"
  echo "$PROMPT"
  echo "── END PROMPT ──"
  exit 0
fi

# ── Call API with retry on 429 ────────────────────────────────────────────────
TMPFILE=$(mktemp /tmp/gpt2_response_XXXXXX.json)
HDRFILE=$(mktemp /tmp/gpt2_headers_XXXXXX.txt)
trap "rm -f $TMPFILE $HDRFILE" EXIT

echo "🎨 Generating ${N} image(s) at ${SIZE} quality=${QUALITY} format=${FORMAT}..."
echo "   Timeout: ${TIMEOUT}s | Deployment: ${DEPLOYMENT} | Max retries: ${MAX_RETRIES}"

ATTEMPT=0
while true; do
  ATTEMPT=$((ATTEMPT + 1))

  if [ $ATTEMPT -gt 1 ]; then
    echo "   Attempt ${ATTEMPT}/${MAX_RETRIES}..."
  fi

  HTTP_CODE=$(curl -s --max-time "$TIMEOUT" -X POST "$URL" \
    -H "api-key: $AZURE_OPENAI_API_KEY" \
    -H "Content-Type: application/json" \
    -d "$BODY" \
    -o "$TMPFILE" -D "$HDRFILE" -w "%{http_code}")

  if [ "$HTTP_CODE" = "200" ]; then
    break
  fi

  if [ "$HTTP_CODE" = "429" ]; then
    if [ $ATTEMPT -ge $MAX_RETRIES ]; then
      echo "ERROR: Rate limited (429) after ${MAX_RETRIES} attempts. Give up." >&2
      cat "$TMPFILE" >&2
      exit 1
    fi

    # Parse Retry-After header (seconds). Fall back to exponential backoff.
    RETRY_AFTER=$(grep -i "^retry-after:" "$HDRFILE" | head -1 | tr -d '[:space:]' | cut -d: -f2)
    if [ -z "$RETRY_AFTER" ] || ! [[ "$RETRY_AFTER" =~ ^[0-9]+$ ]]; then
      RETRY_AFTER=$(( 15 * ATTEMPT ))
    fi

    echo "⏳ Rate limited (429). Waiting ${RETRY_AFTER}s before retry (attempt ${ATTEMPT}/${MAX_RETRIES})..."
    sleep "$RETRY_AFTER"
    continue
  fi

  if [ "$HTTP_CODE" = "000" ]; then
    # curl returns 000 on connection timeout / disconnect
    if [ $ATTEMPT -ge $MAX_RETRIES ]; then
      echo "ERROR: Connection timed out after ${TIMEOUT}s on attempt ${ATTEMPT}. Give up." >&2
      exit 1
    fi
    BACKOFF=$(( 10 * ATTEMPT ))
    echo "⏳ Connection timed out after ${TIMEOUT}s. Retrying in ${BACKOFF}s (attempt ${ATTEMPT}/${MAX_RETRIES})..."
    sleep "$BACKOFF"
    continue
  fi

  # Any other HTTP error — don't retry
  echo "ERROR: HTTP ${HTTP_CODE}" >&2
  cat "$TMPFILE" >&2
  exit 1
done

echo "   HTTP 200 on attempt ${ATTEMPT}"

# ── Decode & save ─────────────────────────────────────────────────────────────
python3 -c "
import json, base64, sys, time, os

with open('$TMPFILE') as f:
    data = json.load(f)

if 'error' in data:
    print(f'ERROR: {json.dumps(data[\"error\"], indent=2)}', file=sys.stderr)
    sys.exit(1)

if 'data' not in data or not data['data']:
    print(f'ERROR: No images in response: {json.dumps(data)[:500]}', file=sys.stderr)
    sys.exit(1)

ts = int(time.time())
saved = []
for i, img in enumerate(data['data']):
    suffix = f'_{i+1}' if len(data['data']) > 1 else ''
    fp = os.path.abspath(f'$OUTPUT_DIR/${LABEL}_{ts}{suffix}.${FORMAT}')
    raw = base64.b64decode(img['b64_json'])
    with open(fp, 'wb') as f:
        f.write(raw)
    mb = len(raw) / 1_048_576
    saved.append(fp)
    print(f'✅ {fp} ({mb:.1f} MB)')

with open('/tmp/gpt2_last_outputs.txt', 'w') as f:
    f.write('\n'.join(saved))
"

RESULT=$?
if [ $RESULT -ne 0 ]; then
  exit $RESULT
fi

# ── Open if requested ─────────────────────────────────────────────────────────
if [ "$OPEN_AFTER" = true ] && [ "$(uname)" = "Darwin" ]; then
  while IFS= read -r fp; do
    open "$fp"
  done < /tmp/gpt2_last_outputs.txt
fi

echo "🏁 Done."
