# DESIGN.md — [[Fluent 2]] Dark for [[maenifold-desktop]]

## 1) Identity

`maenifold-desktop` is a [[Fluent 2]] dark application and a visual sibling of [[agency-cowork]], not a separate “Obsidian Engine” aesthetic experiment.  
This document is normative for Sprint 176 (`T-176.0` through `T-176.15`): downstream screens should mirror the shipped [[agency-cowork]] dark language.

Principles:
- Calm dark layering (not flat black, not glass fantasy).
- Semantic color reserved for meaning, not decoration.
- Real [[Fluent 2]] borders, radii, and shadows.
- Emoji-forward home tiles for fast category recognition.
- Prefer [[shadcn-vue]] primitives over bespoke CSS.

---

## 2) Tokens

Canonical source: `/Users/brett/src/agency-cowork/ui/src/App.jsx` token constant `T` (lines 10–45).

| Token | Value | Usage | Source |
|---|---|---|---|
| `bgApp` | `#1C1C1C` | App canvas/root background | `App.jsx:12` |
| `bgSidebar` | `#282828` | Navigation/sidebar surfaces | `App.jsx:13` |
| `bgSurface` | `#333333` | Standard controls/cards/inputs | `App.jsx:14` |
| `bgSurfaceHover` | `#3A3A3A` | Hover state over interactive surfaces | `App.jsx:15` |
| `bgElevated` | `#2D2D2D` | Elevated tiles/pills/cards | `App.jsx:16` |
| `bgOverlay` | `rgba(0,0,0,0.5)` | Scrim/modal overlay | `App.jsx:17` |
| `borderSubtle` | `#383838` | Standard 1px separators and field borders | `App.jsx:19` |
| `borderCard` | `rgba(255,255,255,0.08)` | Card/tile border on dark surfaces | `App.jsx:20` |
| `textPrimary` | `#FFFFFF` | Primary readable text | `App.jsx:22` |
| `textSecondary` | `#A0AAB2` | Secondary/supporting text | `App.jsx:23` |
| `textTertiary` | `#6B7280` | Captions/meta/subtle labels | `App.jsx:24` |
| `textDisabled` | `#4B5563` | Disabled text/icon state | `App.jsx:25` |
| `brand` | `#0078D4` | Primary actions/focus/accent | `App.jsx:27` |
| `brandHover` | `#106EBE` | Hover/active brand state | `App.jsx:28` |
| `brandLight` | `rgba(0,120,212,0.15)` | Brand-tinted selected backgrounds | `App.jsx:29` |
| `success` | `#0E9C57` | Success/healthy/completed states | `App.jsx:31` |
| `warning` | `#E8910A` | Warning/attention/in-progress states | `App.jsx:32` |
| `danger` | `#D13438` | Error/destructive/failure states | `App.jsx:33` |
| `thinking` | `#B4891C` | Thinking/processing state | `App.jsx:34` |
| `shadow8` | `0 4px 8px rgba(0,0,0,0.24), 0 0 2px rgba(0,0,0,0.12)` | Subtle elevation | `App.jsx:36` |
| `shadow16` | `0 8px 16px rgba(0,0,0,0.28), 0 0 2px rgba(0,0,0,0.12)` | Popovers/raised cards | `App.jsx:37` |
| `shadow64` | `0 32px 64px rgba(0,0,0,0.32), 0 0 8px rgba(0,0,0,0.16)` | Modal-level elevation | `App.jsx:38` |
| `r4` | `4px` | Tight control radius | `App.jsx:40` |
| `r8` | `8px` | Default component radius | `App.jsx:41` |
| `r12` | `12px` | Pills/chips/large soft controls | `App.jsx:42` |
| `font` | `'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, sans-serif` | UI typography stack | `App.jsx:44` |

---

## 3) Typography

### Font Stack (canonical)

`'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, sans-serif` (`App.jsx:44`)

### Type Ramp

| Role | Size / Line Height | Weight | Notes |
|---|---|---|---|
| Display | 32/40 | 600 | Large hero titles |
| H1 | 26/34 | 600 | Home heading (`App.jsx:4737`) |
| H2 | 20/28 | 600 | Panel/section headers |
| H3 | 16/24 | 600 | Card headers |
| H4 | 14/20 | 600 | Compact headings |
| Body | 13–14/20–22 | 400 | Primary body/input text (`App.jsx:5079`) |
| Caption | 11–12/16 | 400–500 | Metadata, tags, helper labels |
| Code | 12/18 | 400 | Monospace where required by content context |

Use `textPrimary`/`textSecondary`/`textTertiary` for hierarchy; do not introduce alternate headline font families.

---

## 4) Surfaces & Elevation

Layering order:
1. `bgApp` (`#1C1C1C`) — app base.
2. `bgSidebar` (`#282828`) — nav rail/sidebar.
3. `bgSurface` (`#333333`) — standard interaction plane.
4. `bgElevated` (`#2D2D2D`) — raised interactive tiles/chips/cards.

Hover state: `bgSurfaceHover` (`#3A3A3A`) for elevated interactive affordances.

Elevation uses **real drop shadows**:
- `shadow8` for subtle lift.
- `shadow16` for popups/menus.
- `shadow64` for modal emphasis.

Explicitly disallowed: ambient-glow-only depth model.

---

## 5) Borders

Obsidian’s “borders prohibited” rule is **reversed**.

[[Fluent 2]] dark uses borders as first-class structure:
- `borderSubtle` (`#383838`) for fields, separators, pane boundaries.
- `borderCard` (`rgba(255,255,255,0.08)`) for cards/tiles on dark surfaces.

Guideline:
- Use `1px` borders by default.
- Use `1.5px` only when emphasizing selected/interactive tiles (as seen in category tiles).

---

## 6) Radii

Canonical radii:
- `4px` (`r4`) for small controls, compact buttons, tags.
- `8px` (`r8`) for default cards, inputs, dropdowns, standard buttons.
- `12px` (`r12`) for pills/chips and larger soft containers.

No large bubbly rounding beyond `12px` unless a true pill/circle control requires it.

---

## 7) Color Roles

| Role | Token | Meaning |
|---|---|---|
| Brand | `brand` / `brandHover` / `brandLight` | Primary action, selected state, focus accent |
| Success | `success` | Completed/healthy/safe |
| Warning | `warning` | Caution/in progress/attention |
| Danger | `danger` | Error, destructive action, blocked/failure |
| Thinking | `thinking` | Agent thinking/processing state |

Semantic colors should encode state or intent; they are not decorative neon accents.

---

## 8) Motion

Match shipped [[agency-cowork]] behavior:
- Default micro-interactions: ~`0.15s` transitions (color/bg/border/transform).
- Secondary toggles and fades: `0.2s`.
- Structural panel width transitions: `0.25s` cubic-bezier (`0.4, 0, 0.2, 1`) where needed.
- Hover affordances may include subtle scale lift (e.g., send/tiles) but no parallax.

Motion should feel responsive and controlled, not theatrical.

---

## 9) Tile Pattern (Homepage)

Canonical pattern is [[agency-cowork]] home category tiles (`App.jsx:4746–4765`):
- Grid layout of category tiles.
- Tile anatomy: **emoji icon + title + subtitle**.
- Background: `bgElevated`.
- Border: `1.5px solid borderCard` (or category tint on hover).
- Hover: `bgSurfaceHover` + subtle highlight/shimmer, preserving readability.

Emoji icons are intentional and canonical for top-level category recognition.

---

## 10) Component Conventions

- Prefer [[shadcn-vue]] components and tokens as the implementation baseline.
- Keep styling token-driven via this document’s [[Fluent 2]] values.
- Avoid net-new bespoke component CSS unless absolutely required.
- Sprint 176 direction: retire legacy bespoke CSS footprint (5,959 LOC referenced in planning) and converge on tokenized primitives.

---

## 11) What Changed from Obsidian Engine

| Obsidian Engine (old) | [[Fluent 2]] dark (new) |
|---|---|
| `surface #131313` monoculture | Layered `bgApp/bgSidebar/bgSurface/bgElevated` |
| “No borders for sectioning” | Borders are allowed and expected (`borderSubtle`, `borderCard`) |
| Ambient glow as primary depth | Real shadow tokens (`shadow8`, `shadow16`, `shadow64`) |
| Space Grotesk + Inter + JetBrains-heavy identity | Segoe UI Variable system stack |
| Single 4px radius doctrine | 4px / 8px / 12px radius scale |
| Stylized futuristic visual mythology | Product-consistent sibling style with [[agency-cowork]] |

This is a deliberate product alignment pivot, not a partial theme tweak.

---

## 12) Source / Provenance

- Primary canonical tokens: `/Users/brett/src/agency-cowork/ui/src/App.jsx` lines `10–45`.
- Supporting implementation references:
  - Home tile pattern: `App.jsx:4746–4765`
  - Focus ring pattern: `App.jsx:6310–6336`, `App.jsx:6876`
  - Motion examples: multiple `transition: ... 0.15s` usages (e.g., `App.jsx:5066`, `App.jsx:5118`)
- Commit for token source file: `0deb7c868caa5ea576a6967ee04f32fc5b71a653`
- Commit date: `Thu Apr 16 00:25:16 2026 +0100`
- Last updated: 2026-04-18
