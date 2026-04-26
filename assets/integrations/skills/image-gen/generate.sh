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
TIMEOUT=300       # 5 min default; bump to 600 for 4K high
OPEN_AFTER=false

# ── Parse args ────────────────────────────────────────────────────────────────
PROMPT=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --size)       SIZE="$2";       shift 2 ;;
    --quality)    QUALITY="$2";    shift 2 ;;
    --format)     FORMAT="$2";     shift 2 ;;
    --n)          N="$2";          shift 2 ;;
    --label)      LABEL="$2";     shift 2 ;;
    --output-dir) OUTPUT_DIR="$2"; shift 2 ;;
    --timeout)    TIMEOUT="$2";    shift 2 ;;
    --transparent) BACKGROUND="transparent"; FORMAT="png"; shift ;;
    --open)       OPEN_AFTER=true; shift ;;
    --help|-h)
      sed -n '3,16p' "$0"
      exit 0 ;;
    @*)
      # Read prompt from file
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

# ── Call API ──────────────────────────────────────────────────────────────────
TMPFILE=$(mktemp /tmp/gpt2_response_XXXXXX.json)
trap "rm -f $TMPFILE" EXIT

echo "🎨 Generating ${N} image(s) at ${SIZE} quality=${QUALITY} format=${FORMAT}..."
echo "   Timeout: ${TIMEOUT}s | Deployment: ${DEPLOYMENT}"

HTTP_CODE=$(curl -s --max-time "$TIMEOUT" -X POST "$URL" \
  -H "api-key: $AZURE_OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d "$BODY" \
  -o "$TMPFILE" -w "%{http_code}")

if [ "$HTTP_CODE" != "200" ]; then
  echo "ERROR: HTTP ${HTTP_CODE}" >&2
  cat "$TMPFILE" >&2
  exit 1
fi

# ── Decode & save ─────────────────────────────────────────────────────────────
python3 -c "
import json, base64, sys, time

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
    fp = f'$OUTPUT_DIR/${LABEL}_{ts}{suffix}.${FORMAT}'
    raw = base64.b64decode(img['b64_json'])
    with open(fp, 'wb') as f:
        f.write(raw)
    mb = len(raw) / 1_048_576
    saved.append(fp)
    print(f'✅ Saved: {fp} ({mb:.1f} MB)')

# Print paths for scripting
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

echo "Done."
