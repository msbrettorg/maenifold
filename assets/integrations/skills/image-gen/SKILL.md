---
name: gpt-image-2
description: Generate or edit images with GPT Image 2 via Azure AI Foundry. Native thinking mode — the model reasons through composition, text placement, and constraints before rendering. Two endpoints — text-to-image (JSON POST) AND image edit (multipart form-data with up to 16 reference images). Best-in-class for fine typography with 95%+ text accuracy across Latin, Japanese, Korean, Chinese, Hindi, Bengali. Sync API, 1-10 images per call, custom dimensions up to 4K/8.3MP. Knowledge cutoff Dec 2025. Includes pointer to a 700+ community prompt library (CC BY 4.0). Triggers on gpt image 2, gpt-image-2, chatgpt image, openai image, azure image, typography image, sign mockup, poster mockup, menu image, packaging mockup, edit image, image edit, modify image, generate image, thinking image, gpt image 2 prompt, awesome gpt-image-2.
---

# GPT Image 2 — Azure AI Foundry

OpenAI's latest image model, hosted on your Azure AI Foundry instance. The strongest model available for **legible text inside images**. Use this when the picture *is* the typography: chalkboards, store signs, hand-lettered posters, menu boards, magazine covers, packaging mockups, UI screenshots, book covers.

## What makes GPT Image 2 different

| Capability | Detail |
|------------|--------|
| **Native Thinking Mode** | The model reasons through composition, object counts, lighting, and constraints **before rendering the first pixel**. Dramatically reduces rerolls for complex briefs. Not a toggle — it's always-on. |
| **95%+ Text Rendering** | Pixel-perfect accuracy for headlines, dense paragraphs, small UI labels, packaging copy. Includes non-Latin scripts: Japanese, Korean, Chinese, Hindi, Bengali. |
| **Multi-Reference Editing** | Accepts up to **16 reference images** on the edit endpoint. Far better character, brand, and material transfer than predecessors. |
| **4K Resolution** | Native up to 3840px (4K). Arbitrary aspect ratios up to 3:1. Custom dimensions as multiples of 16. |
| **Knowledge Cutoff** | December 2025 — generates contextually relevant, factually current content. |
| **Intelligent Routing** | Two routing modes (legacy size tiers and token-bucket selection) let the service auto-select optimal generation config. |

**Two endpoints:**
- **Text-to-image** (`/images/generations`) — generate from a prompt (JSON body)
- **Image edit** (`/images/edits`) — modify uploaded images with optional mask (multipart form-data)

| Field | Value |
|-------|-------|
| Model | `gpt-image-2` |
| Provider | Azure AI Foundry (Azure OpenAI) |
| Resource | `foundy1125` |
| Endpoint | `https://foundy1125.openai.azure.com` |
| Deployment | `gpt-image-2` |
| API Version | `2025-04-01-preview` |
| Auth header | `api-key: $AZURE_OPENAI_API_KEY` |
| Response format | Base64-encoded image data (`b64_json`) — decode and save locally |
| Rate limit | 5 images/min (default quota) |
| Docs | https://learn.microsoft.com/azure/foundry/openai/how-to/dall-e |

---

## Thinking Mode

GPT Image 2 has **native thinking mode** — the model reasons through composition, object counts, lighting, text placement, and constraints before rendering the first pixel. This is not an API parameter you toggle. It's built into the model and always active.

**What thinking mode does:**
- Reasons about layout, composition, and spatial relationships before generating
- Checks its own outputs for consistency (object counts, text accuracy, spatial logic)
- Dramatically reduces the need to reroll complex prompts
- For edits: automatically adjusts reflections, shadows, and color balance to match changes

**How to maximize thinking mode effectiveness:**

1. **Use structured prompts.** JSON-formatted prompts give the model explicit slots for each element, which aligns with how it reasons internally:
   ```json
   {
     "type": "product photography",
     "subject": "matte black ceramic mug with ridge texture",
     "setting": "wet slate countertop, minimalist kitchen, sunrise",
     "camera": "50mm, f/2.0, three-quarter angle, shallow DOF",
     "lighting": "soft directional window light, cool morning tone",
     "text": "MONDAY printed in thin uppercase sans-serif on the side",
     "mood": "calm, editorial"
   }
   ```

2. **Be explicit about constraints.** The model reasons about constraints before rendering — give it clear ones:
   - "Exactly 5 birds on the wire, no more, no fewer"
   - "The sign reads EXACTLY: 'SUMMER COLLECTION' — no other text anywhere"
   - "Three-panel layout, each panel 1:1 square"

3. **Use preserve lists on edits.** Tell the model what must NOT change:
   - "Change the background to night. Preserve: the subject's face, clothing, the brand logo on the bag, the lighting direction on the subject"

4. **Iterate with small edits.** The model's thinking handles incremental changes better than giant rewrites. Chain edit calls rather than trying to get everything in one prompt.

**Thinking mode is NOT the same as the Responses API.** The Responses API lets reasoning models (o3, gpt-5.x) orchestrate image generation as a tool — but on Azure, this currently only supports `gpt-image-1` series, not gpt-image-2. For gpt-image-2, use the direct Images API and let the model's built-in thinking do its work.

---

## Setup

The API key lives in `~/.secrets/azure_openai_api_key` and is loaded by `~/.zprofile` (zsh login shells). **Bash sessions do not source `.zprofile`**, so agents running in bash must load the key explicitly:

```bash
# Load API key if not already set (bash-safe)
if [ -z "$AZURE_OPENAI_API_KEY" ] && [ -r "$HOME/.secrets/azure_openai_api_key" ]; then
  export AZURE_OPENAI_API_KEY="$(cat "$HOME/.secrets/azure_openai_api_key")"
fi
```

Env vars already available in all shells (inherited from parent process):
- `AZURE_OPENAI_ENDPOINT` — `https://foundy1125.openai.azure.com`
- `AZURE_RESOURCE_NAME` — `foundy1125`

The image deployment name `gpt-image-2` is **not** in an env var (the `AZURE_OPENAI_DEPLOYMENT_NAME` var points to the chat model). Hardcode it or set your own.

---

## How to call it

### Text-to-image — minimum request

```bash
# Source the key first (see Setup)
source ~/.zprofile 2>/dev/null || export AZURE_OPENAI_API_KEY="$(cat ~/.secrets/azure_openai_api_key)"

curl -s -X POST \
  "${AZURE_OPENAI_ENDPOINT}/openai/deployments/gpt-image-2/images/generations?api-version=2025-04-01-preview" \
  -H "api-key: $AZURE_OPENAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "A vintage diner chalkboard reading TODAY SPECIAL — Lobster Roll $24", "n": 1, "size": "1536x1024", "quality": "medium"}' \
  | python3 -c "
import sys, json, base64
data = json.load(sys.stdin)
for i, img in enumerate(data['data']):
    with open(f'output_{i}.png', 'wb') as f:
        f.write(base64.b64decode(img['b64_json']))
    print(f'Saved: output_{i}.png')
"
```

### Text-to-image — full request body

```json
{
  "prompt": "...",
  "n": 1,
  "size": "1536x1024",
  "quality": "medium",
  "output_format": "png",
  "background": "auto"
}
```

### Image edit — minimum request

Upload the source image as multipart form-data. Prompt describes the edit.

```bash
curl -s -X POST \
  "${AZURE_OPENAI_ENDPOINT}/openai/deployments/gpt-image-2/images/edits?api-version=2025-04-01-preview" \
  -H "api-key: $AZURE_OPENAI_API_KEY" \
  -F "image[]=@source_photo.png" \
  -F 'prompt=Same workers, same beam — but they are all on phones now. One taking a selfie, one annoyed on a call. Hard hats with AirPods.' \
  -F "size=1024x1024" \
  -F "quality=medium" \
  | python3 -c "
import sys, json, base64
data = json.load(sys.stdin)
for i, img in enumerate(data['data']):
    with open(f'edited_{i}.png', 'wb') as f:
        f.write(base64.b64decode(img['b64_json']))
    print(f'Saved: edited_{i}.png')
"
```

### Image edit — multi-reference (up to 16 images)

GPT Image 2 accepts up to 16 reference images on the edit endpoint. Tell the model what to use each reference for:

```bash
curl -s -X POST \
  "${AZURE_OPENAI_ENDPOINT}/openai/deployments/gpt-image-2/images/edits?api-version=2025-04-01-preview" \
  -H "api-key: $AZURE_OPENAI_API_KEY" \
  -F "image[]=@character_ref.png" \
  -F "image[]=@outfit_ref.png" \
  -F "image[]=@pose_ref.png" \
  -F 'prompt=Generate a new scene with the exact same character (Image 1), wearing the outfit from Image 2, posed as in Image 3, in a rainy Tokyo street at night. Preserve face, hair, fabric texture exactly. Cinematic lighting.' \
  -F "size=1536x1024" \
  -F "quality=high"
```

### Image edit — with mask

```bash
curl -s -X POST \
  "${AZURE_OPENAI_ENDPOINT}/openai/deployments/gpt-image-2/images/edits?api-version=2025-04-01-preview" \
  -H "api-key: $AZURE_OPENAI_API_KEY" \
  -F "image[]=@source_photo.png" \
  -F "mask=@mask.png" \
  -F 'prompt=Replace the sky with a dramatic sunset' \
  -F "size=1024x1024" \
  -F "quality=high" \
  -F "n=1"
```

- The **mask** must be a PNG with the same dimensions as the input image. Fully transparent pixels (alpha=0) mark the region to edit.

### Response (both endpoints)

Azure always returns base64-encoded image data, **not URLs**:

```json
{
  "created": 1698116662,
  "data": [
    {
      "b64_json": "<base64 image data>"
    }
  ]
}
```

**You must decode `b64_json` and write to disk.** No expiring URLs to worry about — the data is inline.

---

## Parameters

### Text-to-image (`/images/generations`)

| Param | Required | Options | Default |
|-------|----------|---------|---------|
| `prompt` | Yes | string | — |
| `size` | No | `"{width}x{height}"` (multiples of 16) | `"1024x1024"` |
| `quality` | No | `"low"`, `"medium"`, `"high"` | `"high"` |
| `n` | No | 1 – 10 | `1` |
| `output_format` | No | `"png"`, `"jpeg"` (no WEBP on Azure) | `"png"` |
| `output_compression` | No | 0 – 100 (JPEG only) | — |
| `background` | No | `"auto"`, `"transparent"` | `"auto"` |
| `stream` | No | bool — receive partial images as SSE | `false` |
| `partial_images` | No | 0 – 3 (requires `stream: true`) | `0` |
| `user` | No | string — unique user ID for usage tracking | — |

### Image edit (`/images/edits`)

All generation params above, plus:

| Param | Required | Format | Notes |
|-------|----------|--------|-------|
| `image[]` | Yes | file upload (multipart) | PNG or JPG, < 50 MB. Up to **16 reference images**. |
| `prompt` | Yes | string — describe the edit | Describe change + preserve list |
| `mask` | No | file upload (PNG, same dimensions) | Transparent pixels = edit region |
| `input_fidelity` | No | string | Controls how closely the model matches input style/faces |

**Content-Type for edits is `multipart/form-data`**, not JSON.

### Image sizes (gpt-image-2)

Arbitrary resolutions with these constraints:
- Both dimensions must be multiples of **16**
- Max edge: **3840px** (4K support)
- Aspect ratio: **≤ 3:1**
- Total pixels between **655,360 and 8,294,400** (~640x1024 min, ~3840x2160 max)

Common presets: `1024x1024`, `1536x1024`, `1024x1536`.

### Intelligent routing

GPT Image 2 includes an intelligent routing layer. If you omit the `size` parameter, the service selects the optimal configuration automatically using one of two modes:

- **Legacy size tiers:** `smimage`, `image`, `xlimage`
- **Token-bucket selection:** Buckets 16, 24, 36, 48, 64, 96 — maps roughly to the size tiers and optimizes output quality/efficiency per prompt.

---

## Transparent backgrounds

The Azure docs explicitly list `background: "transparent"` as a supported parameter for both gpt-image-1 and gpt-image-2 in the prose documentation. However, the SDK code samples annotate it as "(GPT-image-1 only)". The docs are self-contradictory on this point.

**Use it — but verify the output has a real alpha channel:**

```json
{
  "prompt": "A cute robot mascot on a transparent background",
  "background": "transparent",
  "output_format": "png",
  "size": "1024x1024"
}
```

Requires `output_format: "png"`. Check the output PNG actually has an alpha channel (not a fake checkerboard pattern painted into RGB pixels). If transparency doesn't work for gpt-image-2 on your deployment, chain a background remover: Azure AI Vision background removal API, or locally the `rembg` Python package.

---

## Streaming

Get partial images as they generate (useful for showing progress):

```json
{
  "prompt": "...",
  "stream": true,
  "partial_images": 3,
  "size": "1024x1024"
}
```

Returns SSE events with intermediate `b64_json` data before the final image.

---

## Responses API path (reasoning-model orchestrated)

An alternative to the direct Images API is the **Responses API**, where a reasoning model (o3, gpt-5.x) orchestrates image generation as a tool. The reasoning model thinks through the prompt, potentially across multiple steps, before dispatching to the image model.

**Current limitation on Azure:** The Responses API image generation tool **only supports `gpt-image-1` series models**, not gpt-image-2. When gpt-image-2 support is added, this becomes the path for explicit chain-of-thought + image generation.

**How it works** (with gpt-image-1 today, gpt-image-2 in the future):

```python
from openai import OpenAI

client = OpenAI(
  base_url=f"{ENDPOINT}/openai/v1/",
  api_key=API_KEY,
  default_headers={
    "x-ms-oai-image-generation-deployment": "gpt-image-1.5",
    "api_version": "preview"
  }
)

response = client.responses.create(
    model="o3",  # reasoning model orchestrates
    input="Generate an image of a gray tabby cat hugging an otter with an orange scarf",
    tools=[{"type": "image_generation"}],
)
```

The reasoning model thinks through the request, then calls the image model as a tool. This gives you explicit reasoning steps in the response alongside the generated image. Watch the Azure docs for gpt-image-2 support.

---

## Prompting guide

### The formula

GPT Image 2 is a natural-language model with real reasoning. You no longer need keyword-stuffed spam like "8k masterpiece trending on artstation." The model rewards clear, structured description.

```
[Subject + Adjectives] doing [Action] in [Scene/Context].
[Composition/Camera]. [Lighting/Atmosphere].
[Style/Medium]. [Exact Text + Typography].
[Aspect Ratio/Use Case].
```

### Example breakdown

- **Subject:** A matte black ceramic coffee mug with a subtle ridge texture
- **Action:** sitting on a wet slate countertop next to a folded linen napkin
- **Scene:** in a minimalist Scandinavian kitchen at sunrise
- **Composition:** three-quarter angle, 50mm lens, shallow depth of field
- **Lighting:** soft directional window light, cool morning tone, gentle rim highlight
- **Style:** editorial product photography, natural film grain
- **Text:** The mug has MONDAY printed in thin uppercase sans-serif on the side, perfectly legible

### Prompt templates

These templates work with the Azure Images API. Based on community-tested patterns.

**Product ad:**
```
Create a clean, sell-ready product ad for [PRODUCT]. Studio lighting on a soft
neutral backdrop, subtle drop shadow, centered composition. Add a short tagline
in the top-left in modern sans-serif: '[TAGLINE]'. High quality mode.
```

**Text-perfect e-commerce shot:**
```
A premium product shot of [PRODUCT] on a textured surface. Natural window light,
photoreal materials. The front label reads EXACT TEXT: '[BRAND]' in the top third
and '[VARIANT]' below it. No extra words, no duplicate text, perfectly kerned
serif type.
```

**Brand guideline sheet (from a photo reference):**
```
Using the attached photo as inspiration, generate a complete brand guideline sheet
for [BUSINESS TYPE]. Include: logo (primary + mono), color palette with hex codes,
typography pairing, a sample social post, and a packaging mockup. All on a single
clean layout. Legible, production-grade.
```

**Fact-grounded infographic:**
```
Generate a step-by-step infographic showing how to [TOPIC]. Use real, accurate
information. Clean vector style, pastel palette, numbered steps with icons and
short labels. Title at the top: '[TITLE]'. 9:16 vertical for social sharing.
```

**Cinematic ultra-wide concept art:**
```
Ultra-wide 21:9 cinematic concept art: [SCENE DESCRIPTION]. Volumetric light,
atmospheric haze, matte-painting style, hyper-detailed foreground with soft
background falloff. Epic scale.
```

**Structured JSON portrait (most repeatable):**
```json
{
  "subject": "30yo woman, natural makeup, direct gaze",
  "wardrobe": "oversized cream knit sweater",
  "setting": "minimalist apartment, soft afternoon light from left",
  "camera": "50mm, f/2.0, eye-level, 3/4 body framing",
  "mood": "calm, candid, unposed",
  "grading": "muted natural tones, slight filmic grain"
}
```

### Prompt neutrality — mandatory preview

**Agents MUST preview the prompt to the user before calling the API.** Show the exact prompt text and all parameters (size, quality, n). Wait for approval. Do not inject style biases, aesthetic nudges, or creative direction that the user didn't ask for. The model's thinking mode will make its own creative decisions — your job is to faithfully translate the user's intent, not art-direct on their behalf.

**Agents MUST always save the image and report the file path.** Never discard output to `/dev/null` or skip saving. Every API call costs money and time — the output is always worth keeping. Use `generate.sh` which handles this automatically.

Bad: User says "visual endophasia" → agent adds "hieroglyphic narrative, ancient Egyptian symbols, glowing luminous imagery"
Good: User says "visual endophasia" → agent writes "A visual representation of interconnected thought clusters, rendered as imagery rather than text" and lets the model decide the style.

If the user provides style direction ("like hieroglyphs", "cyberpunk", "watercolor"), include it verbatim. If they don't, don't invent it.

### Typography-specific tips

- **Quote the text:** `hand-lettered text reads "OPEN AT 7"` — quoting locks in the literal characters.
- **Specify the lettering style:** `chalk lettering`, `serif sign painter`, `bold sans-serif neon`, `embossed metal type`, `cursive script`.
- **Anchor the surface:** chalkboard, brick wall, neon storefront, paper menu, vinyl record sleeve.
- **Mention lighting:** `warm tungsten`, `golden hour`, `cool fluorescent`.
- **Layout hints work:** `the title stacked over three lines`, `with a small drawing below`, `centered`.
- **For multi-line text:** spell out line breaks. `Line 1: "ESPRESSO". Line 2: "$4.50".` Don't trust newlines.
- **Multilingual text is supported:** Japanese, Korean, Chinese, Hindi, Bengali with 95%+ accuracy. Specify the script explicitly.

### Edit-specific tips

- **Describe the change, not the whole scene.** "Same workers, same beam — but everyone is on their phone now" beats re-describing the original.
- **Use a preserve list.** Explicitly call out what must NOT change: "Preserve: the subject's face, clothing color, brand logo, lighting direction."
- **Use a mask for surgical edits.** Without a mask, the model may reinterpret the whole image.
- **Chain edits.** Save the previous output, upload it as the new input. Small iterative edits consistently beat one giant rewrite.
- **Use `input_fidelity` for face preservation.** Higher fidelity better preserves facial features during edits.
- **Multi-reference:** upload up to 16 images and tell the model what each one contributes: "Image 1 = face, Image 2 = outfit, Image 3 = pose reference."

### Common mistakes

- **Keyword spam.** `8k masterpiece trending on artstation` does nothing. GPT Image 2 reads natural language, not 2023 Midjourney tags.
- **Vague text instructions.** Don't say "add a title." Say: `EXACT TEXT: "SUMMER COLLECTION" in bold uppercase serif, centered, white on charcoal.`
- **Assuming 4K is always better.** Resolutions above 2K are flagged as experimental. For most work: generate at `quality=medium` + upscale separately if needed.
- **Forgetting the preserve list.** Without it, the model may drift on faces, logos, or text you wanted kept.
- **Over-prompting edits.** Don't re-describe the entire scene — describe only what changes.

---

## When to use vs other models

| Scenario | Endpoint | Notes |
|----------|----------|-------|
| Sign / chalkboard / poster with text | generations | Best in class for typography |
| Magazine cover / book cover | generations | |
| Menu board / packaging mockup | generations | |
| UI screenshot mockup | generations | |
| Brand guideline sheet | generations (structured JSON prompt) | |
| Multilingual text (CJK, Hindi, Bengali) | generations | 95%+ accuracy |
| Modify an existing image | edits | Upload image as form-data |
| Multi-reference character consistency | edits (up to 16 refs) | Specify what each reference contributes |
| Surgical region edit ("only change this corner") | edits + mask | Mask defines the editable region |
| Iterative refinement | edits | Save previous output, upload as new input |
| Photoreal portrait | other | Use a dedicated portrait model |
| Need > 5 images/min sustained | other | Azure default quota is 5/min |
| Explicit chain-of-thought + image gen | Responses API (gpt-image-1 only today) | Wait for gpt-image-2 Responses API support |

---

## generate.sh — the script

A batteries-included script lives alongside this SKILL.md at `generate.sh`. It handles API key loading, JSON assembly, the API call, 429 rate-limit retries with `Retry-After` header parsing, connection timeout retries, base64 decoding, file saving, and optional Preview.app launch.

**Quick examples:**

```bash
# Simple generation
./generate.sh "A chalkboard reading OPEN AT 7" --open

# 4K high quality (needs longer timeout — generation takes 2-5 min)
./generate.sh "Epic landscape" --size 3840x2160 --quality high --timeout 600 --label hero --open

# Read prompt from file
./generate.sh @prompt.txt --size 1536x1024 --quality medium --open

# Multiple variants
./generate.sh "Product shot of a coffee mug" --n 4 --label mug --open

# Transparent background
./generate.sh "Robot mascot" --transparent --open

# Custom output dir
./generate.sh "Poster art" --output-dir ~/Desktop/images --label poster
```

**All flags:**

| Flag | Default | Description |
|------|---------|-------------|
| `--size WxH` | `1536x1024` | Output dimensions (multiples of 16) |
| `--quality` | `medium` | `low`, `medium`, `high` |
| `--format` | `png` | `png`, `jpeg` |
| `--n` | `1` | Number of images (1-10) |
| `--label` | `gpt2` | Filename prefix |
| `--output-dir` | `./output` | Where to save |
| `--timeout` | `300` | Curl timeout in seconds. **Use 600 for 4K high.** |
| `--retries` | `5` | Max retry attempts on 429 / timeout |
| `--dry-run` | off | Preview prompt and params without calling the API. **Agents must use this first.** |
| `--transparent` | off | Sets `background: transparent` + forces PNG |
| `--open` | off | Open in Preview.app after saving (macOS) |
| `@file.txt` | — | Read prompt from a file |

**Retry behavior:**
- **HTTP 429 (rate limited):** Reads `Retry-After` header for wait time. Falls back to `15s × attempt` if header missing. Retries up to `--retries` times.
- **Connection timeout (HTTP 000):** Retries with `10s × attempt` backoff. Common with 4K + high quality which can take 2-5 minutes server-side.
- **Other HTTP errors:** Fails immediately with the error body.

**Agents: use the script, don't roll your own curl.** The script path is available at the skill symlink:
```bash
~/.copilot/skills/image-gen/generate.sh "your prompt" --open
```

---

## Recipe — Python

```python
import os, time, json, base64, pathlib, requests

# Load API key — handles both zsh (env var set) and bash (needs file read)
API_KEY = os.environ.get('AZURE_OPENAI_API_KEY') or \
    open(os.path.expanduser('~/.secrets/azure_openai_api_key')).read().strip()

ENDPOINT = os.environ.get('AZURE_OPENAI_ENDPOINT', 'https://foundy1125.openai.azure.com')
DEPLOYMENT = 'gpt-image-2'
API_VERSION = '2025-04-01-preview'
BASE_URL = f'{ENDPOINT}/openai/deployments/{DEPLOYMENT}/images'
HEADERS = {'api-key': API_KEY}


def decode_and_save(data, out_dir='./output', label='gpt2', fmt='png'):
    """Decode b64_json responses and save to disk."""
    pathlib.Path(out_dir).mkdir(parents=True, exist_ok=True)
    ts = int(time.time() * 1000)
    saved = []
    for i, img in enumerate(data['data']):
        suffix = f'_{i+1}' if len(data['data']) > 1 else ''
        fp = f'{out_dir}/{label}_{ts}{suffix}.{fmt}'
        with open(fp, 'wb') as f:
            f.write(base64.b64decode(img['b64_json']))
        saved.append(fp)
        print(f'Saved: {fp}')
    return saved


def generate(prompt, size='1536x1024', quality='medium', n=1,
             output_format='png', background='auto', **kwargs):
    """Text-to-image generation."""
    url = f'{BASE_URL}/generations?api-version={API_VERSION}'
    body = {
        'prompt': prompt, 'size': size, 'quality': quality,
        'n': n, 'output_format': output_format, 'background': background,
    }
    res = requests.post(url, headers={**HEADERS, 'Content-Type': 'application/json'},
                        json=body)
    data = res.json()
    if 'error' in data:
        raise RuntimeError(f'API error: {data["error"]}')
    return decode_and_save(data, fmt=output_format, **kwargs)


def edit(prompt, image_paths, mask_path=None, size='1024x1024',
         quality='medium', n=1, **kwargs):
    """Edit existing images via multipart upload. Accepts up to 16 reference images."""
    url = f'{BASE_URL}/edits?api-version={API_VERSION}'
    if isinstance(image_paths, str):
        image_paths = [image_paths]
    files = [('image[]', (os.path.basename(p), open(p, 'rb'), 'image/png'))
             for p in image_paths]
    if mask_path:
        files.append(('mask', (os.path.basename(mask_path),
                               open(mask_path, 'rb'), 'image/png')))
    form_data = {'prompt': prompt, 'size': size, 'quality': quality, 'n': str(n)}
    res = requests.post(url, headers=HEADERS, files=files, data=form_data)
    data = res.json()
    if 'error' in data:
        raise RuntimeError(f'API error: {data["error"]}')
    return decode_and_save(data, **kwargs)


# --- Examples ---

# Text-to-image
generate(prompt='A chalkboard reading "OPEN AT 7"', label='chalkboard')

# Structured JSON prompt (best for complex scenes)
generate(
    prompt=json.dumps({
        "type": "product photography",
        "subject": "matte black ceramic mug",
        "setting": "wet slate countertop, minimalist kitchen",
        "camera": "50mm, f/2.0, three-quarter angle",
        "text": "MONDAY in thin uppercase sans-serif"
    }),
    label='structured',
)

# Multi-reference edit (character + outfit + pose)
edit(
    prompt='Same character from Image 1, wearing outfit from Image 2, posed as Image 3, in a rainy Tokyo street',
    image_paths=['./character.png', './outfit.png', './pose.png'],
    label='multi_ref',
)

# Single-image edit
edit(
    prompt='Same scene but everyone is on their phone now',
    image_paths='./source.png',
    label='phones_edit',
)
```

## Recipe — Node.js

```javascript
import fs from 'node:fs';
import path from 'node:path';

// Load API key — handles both zsh (env var set) and bash (needs file read)
const API_KEY = process.env.AZURE_OPENAI_API_KEY ||
  fs.readFileSync(path.join(process.env.HOME, '.secrets/azure_openai_api_key'), 'utf8').trim();

const ENDPOINT = process.env.AZURE_OPENAI_ENDPOINT || 'https://foundy1125.openai.azure.com';
const DEPLOYMENT = 'gpt-image-2';
const API_VERSION = '2025-04-01-preview';
const BASE_URL = `${ENDPOINT}/openai/deployments/${DEPLOYMENT}/images`;

function decodeAndSave(data, { outDir = './output', label = 'gpt2', fmt = 'png' } = {}) {
  fs.mkdirSync(outDir, { recursive: true });
  const ts = Date.now();
  const saved = [];
  for (let i = 0; i < data.data.length; i++) {
    const suffix = data.data.length > 1 ? `_${i + 1}` : '';
    const fp = path.join(outDir, `${label}_${ts}${suffix}.${fmt}`);
    fs.writeFileSync(fp, Buffer.from(data.data[i].b64_json, 'base64'));
    saved.push(fp);
    console.log(`Saved: ${fp}`);
  }
  return saved;
}

// Text-to-image
async function generate({
  prompt, size = '1536x1024', quality = 'medium', n = 1,
  output_format = 'png', background = 'auto', outDir, label,
}) {
  const res = await fetch(`${BASE_URL}/generations?api-version=${API_VERSION}`, {
    method: 'POST',
    headers: { 'api-key': API_KEY, 'Content-Type': 'application/json' },
    body: JSON.stringify({ prompt, size, quality, n, output_format, background }),
  });
  const data = await res.json();
  if (data.error) throw new Error(`API error: ${JSON.stringify(data.error)}`);
  return decodeAndSave(data, { outDir, label, fmt: output_format });
}

// Image edit — supports multiple reference images (up to 16)
async function edit({
  prompt, imagePaths, maskPath, size = '1024x1024',
  quality = 'medium', n = 1, outDir, label,
}) {
  const paths = Array.isArray(imagePaths) ? imagePaths : [imagePaths];
  const form = new FormData();
  for (const p of paths) {
    form.append('image[]', new Blob([fs.readFileSync(p)]), path.basename(p));
  }
  if (maskPath) {
    form.append('mask', new Blob([fs.readFileSync(maskPath)]), path.basename(maskPath));
  }
  form.append('prompt', prompt);
  form.append('size', size);
  form.append('quality', quality);
  form.append('n', String(n));

  const res = await fetch(`${BASE_URL}/edits?api-version=${API_VERSION}`, {
    method: 'POST',
    headers: { 'api-key': API_KEY },
    body: form,
  });
  const data = await res.json();
  if (data.error) throw new Error(`API error: ${JSON.stringify(data.error)}`);
  return decodeAndSave(data, { outDir, label });
}

// --- Examples ---

await generate({
  prompt: 'A chalkboard sign on a brick wall, hand-lettered: "OPEN AT 7"',
  label: 'chalkboard',
});

await edit({
  prompt: 'Same character, wearing the outfit from Image 2, in a rainy street',
  imagePaths: ['./character.png', './outfit.png'],
  label: 'multi_ref',
});
```

---

## Pricing

Azure OpenAI image generation is token-based. Per the Microsoft Foundry announcement:

| Token type | Input | Cached input | Output |
|------------|-------|--------------|--------|
| Image tokens | $8 / 1M | $2 / 1M | $30 / 1M |
| Text tokens | $5 / 1M | $1.25 / 1M | $10 / 1M |

**Rough per-image cost guidance:**

| Quality | Relative cost | When to use |
|---------|--------------|-------------|
| `low` | ~$0.01/image | Cheap drafts, rapid iteration |
| `medium` | ~$0.05/image | Sweet spot for most work |
| `high` | ~$0.18-0.41/image | Final output where detail matters |

`quality` is the biggest cost lever. Default to `medium`. Bump to `high` only for finals. `low` for cheap exploration. Check actual usage in the Azure portal under your OpenAI resource's monitoring blade.

---

## Notes / gotchas

- **Thinking mode is always-on.** There is no API toggle. The model reasons before rendering. Structured prompts and explicit constraints help it reason more effectively.
- **Sync API** — single POST returns base64 image data inline. No polling needed. Takes 10-30s for standard sizes; **2-5 minutes for 4K + high quality**. Use `--timeout 600` in generate.sh.
- **429 rate limiting** — Azure enforces 5 images/min default quota. The generate.sh script auto-retries with `Retry-After` header parsing. If you're calling the API directly, check for 429 and back off.
- **Auth header is `api-key`** — Azure-specific. Not `Bearer`, not `Authorization: Key`.
- **Response is `b64_json`, not a URL.** Decode base64 and write to disk. No expiring URLs.
- **Up to 10 images per call** via `n`. Cost scales linearly.
- **Up to 16 reference images on edits.** Specify what each reference contributes in the prompt.
- **Edits use multipart form-data** with file uploads, not JSON with URLs.
- **Edit mask** must be a PNG with the same dimensions as the input. Transparent pixels (alpha=0) mark the editable region.
- **Input image limit:** < 50 MB, PNG or JPG only.
- **Transparent backgrounds** — `background: "transparent"` is documented for both gpt-image-1 and gpt-image-2 in Azure docs (though SDK code comments say "gpt-image-1 only"). Verify the output PNG has a real alpha channel.
- **Streaming** — `stream: true` + `partial_images: 1-3` for progressive rendering.
- **Rate limit is 5 images/min** by default. Request a quota increase via Azure portal.
- **Custom sizes:** multiples of 16, total pixels in [655K, 8.3M], aspect ratio ≤ 3:1, max edge 3840px.
- **Bash sessions need explicit key loading** — `AZURE_OPENAI_API_KEY` is not in bash env by default. See Setup section.
- **Chaining edits:** save previous output to disk, upload as new input. No URL passing.
- **Knowledge cutoff is December 2025.** The model generates contextually current content.
- **Multilingual text:** Japanese, Korean, Chinese, Hindi, Bengali with 95%+ accuracy.
- **Responses API** supports reasoning-model orchestrated image gen, but currently only gpt-image-1 on Azure. Watch for gpt-image-2 support.
- **Keyword spam does nothing.** "8k masterpiece trending on artstation" is ignored. Use clear natural language.
- **No WEBP on Azure.** Only PNG and JPEG are supported. WEBP requests will fail.
- **Resolutions above 2K are experimental.** For production, generate at medium quality + upscale separately.

---

## Prompt examples library

For style/category inspiration, the **`awesome-gpt-image-2`** community library has 700+ curated prompts (CC BY 4.0, attribution required). Use it whenever a user asks "how would I prompt this?" for a recognizable style or use-case.

**Two ways in:**

1. **Browse the gallery (humans):** [youmind.com/gpt-image-2-prompts](https://youmind.com/gpt-image-2-prompts) — masonry grid, full-text search, deep-linkable categories.
2. **Grep the README (agents):** [raw.githubusercontent.com/YouMind-OpenLab/awesome-gpt-image-2/main/README.md](https://raw.githubusercontent.com/YouMind-OpenLab/awesome-gpt-image-2/main/README.md) — single markdown file, ~350KB, all prompts in one place.

**Category slugs** (use as `?categories={slug}` on youmind.com, or as a grep target in the README):

- **Use cases:** `profile-avatar`, `social-media-post`, `infographic-edu-visual`, `youtube-thumbnail`, `comic-storyboard`, `product-marketing`, `ecommerce-main-image`, `game-asset`, `poster-flyer`, `app-web-design`
- **Styles:** `photography`, `cinematic-film-still`, `anime-manga`, `illustration`, `sketch-line-art`, `comic-graphic-novel`, `3d-render`, `chibi-q-style`, `isometric`, `pixel-art`, `oil-painting`, `watercolor`, `ink-chinese-style`, `retro-vintage`, `cyberpunk-sci-fi`, `minimalism`
- **Subjects:** `portrait-selfie`, `influencer-model`, `character`, `group-couple`, `product`, `food-drink`, `fashion-item`, `animal-creature`, `vehicle`, `architecture-interior`, `landscape-nature`, `cityscape-street`, `diagram-chart`, `text-typography`, `abstract-background`

**Recipe — fetch + grep examples for a category:**

```bash
curl -s https://raw.githubusercontent.com/YouMind-OpenLab/awesome-gpt-image-2/main/README.md -o /tmp/gpt2_prompts.md
grep -n "^### No\." /tmp/gpt2_prompts.md | grep -i "youtube thumbnail"
sed -n '2722,2830p' /tmp/gpt2_prompts.md
```

**Attribution.** When reusing a prompt verbatim, credit YouMind OpenLab + the original author. Repo: [github.com/YouMind-OpenLab/awesome-gpt-image-2](https://github.com/YouMind-OpenLab/awesome-gpt-image-2).

---

## Links

- Azure OpenAI image generation guide: https://learn.microsoft.com/azure/foundry/openai/how-to/dall-e
- Azure OpenAI Responses API: https://learn.microsoft.com/azure/foundry/openai/how-to/responses
- Azure OpenAI quickstart: https://learn.microsoft.com/azure/foundry/openai/dall-e-quickstart
- Image prompt engineering: https://learn.microsoft.com/azure/ai-foundry/openai/concepts/gpt-4-v-prompt-engineering
- Quota management: https://learn.microsoft.com/azure/ai-foundry/openai/how-to/quota
- Microsoft Foundry blog announcement: https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/introducing-openais-gpt-image-2-in-microsoft-foundry/4500571
- Atlabs prompting guide: https://www.atlabs.ai/blog/the-ultimate-gpt-image-2-prompting-guide
- Prompt library (gallery): https://youmind.com/gpt-image-2-prompts
- Prompt library (repo, CC BY 4.0): https://github.com/YouMind-OpenLab/awesome-gpt-image-2
