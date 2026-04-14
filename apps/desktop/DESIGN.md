# DESIGN.md — Obsidian Engine

> Reference: Stitch project `projects/1858563870757498867` (21 screens)
> Source: `memory://pipeline-ui/the-kinetic-lumina-pipeline-ui-design-system-vision`

## North Star

**"The Kinetic Monolith"** — a high-performance console carved from dark, translucent materials. Not a web page. Not a dashboard. A precision instrument.

---

## 1. Surface Hierarchy (No-Line Rule)

Hard 1px borders are **prohibited** for sectioning. Boundaries are defined through background color shifts only.

| Token | Hex | Tailwind | Role |
|-------|-----|----------|------|
| `surface` | `#131313` | `bg-background` | Canvas / base layer |
| `surface-container-lowest` | `#0e0e0e` | — | Recessed canvas (graph, deep wells) |
| `surface-container-low` | `#1c1b1b` | — | Sidebars, panels, sectioning |
| `surface-container` | `#201f1f` | `bg-card` | Cards, dialogs, interactive containers |
| `surface-container-high` | `#2a2a2a` | `bg-muted` | Hover states, active items |
| `surface-container-highest` | `#353534` | — | Floating: tooltips, context menus |

### Ghost Border

When accessibility requires containment: `1px` stroke of `outline-variant` (`#424753`) at **15% opacity**. Felt, not seen.

```css
border: 1px solid rgba(66, 71, 83, 0.15);
```

### Ambient Glow (modals/floating)

No drop shadows. Use blue-tinted ambient glow:
```css
box-shadow: 0 0 32px rgba(173, 198, 255, 0.08);
```

---

## 2. Color Accents

| Name | Hex | Use |
|------|-----|-----|
| Workflow Blue | `#4d8eff` | Primary actions, CTA buttons, active indicators |
| Prompt Blue | `#adc6ff` | User inputs, text triggers, secondary highlights |
| Thinking Purple | `#d0bcff` | Processing states, AI thinking indicators |
| Tool Emerald | `#4edea3` | Success, health, completed states |
| Amber | `#F59E0B` | Active/in-progress, warnings |
| Error Red | `#ffb4ab` | Errors, destructive actions |

### Status Beads

6px circles, not banners:
- Ready: `#4edea3` (emerald)
- Active: `#F59E0B` (amber)
- Error: `#ffb4ab` (red)
- Pending: `#707070` (muted)

---

## 3. Typography

| Use | Family | Weight | Size | Token |
|-----|--------|--------|------|-------|
| Headlines | Space Grotesk | 500 | 1.125rem–1.5rem | `font-display` |
| App title | Space Grotesk | 500 | 0.875rem | — |
| Body | Inter | 400 | 0.875rem | `font-body` / `text-body` |
| Labels | Inter | 400 | 0.6875rem | `text-tiny` |
| Code/IDs | JetBrains Mono | 400 | 0.75rem | `font-code` |

### Text Colors

| Role | Hex | Token |
|------|-----|-------|
| Primary text | `#e5e2e1` | `text-foreground` |
| Secondary text | `#c2c6d6` | `text-muted-foreground` |
| Muted/labels | `#707070` | — |

### Rules

- **Sentence case always.** Never ALL CAPS for labels or headers.
- No 100% white (`#ffffff`). Use `#e5e2e1` for maximum contrast.
- No Title Case for labels. Sentence case only.

---

## 4. Components

### Buttons

| Variant | Background | Border | Text |
|---------|-----------|--------|------|
| Primary | Gradient `#adc6ff` → `#4d8eff` | none | `#001a42` (dark) |
| Secondary/Ghost | `#2a2a2a` | Ghost border | `#e5e2e1` |
| Tertiary | transparent | Ghost border | `#adc6ff` |
| Destructive | transparent | Ghost border | `#ffb4ab` |

Radius: `0.25rem` (4px). Tight, not bubbly.

### Inputs

- Base: `#1c1b1b` background, no border
- Focus: shift to `#2a2a2a` + blue ghost border at 40% opacity
- Placeholder: `#707070`

### Cards & Lists

- No divider lines between items. Use `0.5rem`–`1rem` gap.
- Hover: shift from `#201f1f` → `#2a2a2a`
- Selected: `#2a2a2a` + blue left accent bar (4px)

### Pipeline Nodes

- Body: `#201f1f`, no border
- 4px left accent bar colored by type (blue/purple/emerald/amber)
- Active node: ambient glow at 10% opacity, 20px blur
- Status dot: 6px bead in header

### Dialogs/Modals

- Background: `#201f1f`, 8px radius
- Ambient blue glow shadow
- Backdrop: `#000000` at 60% opacity
- Footer: primary action right-aligned, ghost cancel left

---

## 5. Layout Patterns

### Sidebar (Agents view)
- Width: 280px, background: `#1c1b1b`
- One cohesive surface — no nested bordered boxes
- Compact config rows: label + dropdown, no redundant value echo
- Primary CTA (New Session) at bottom, filled blue

### Pipeline Canvas
- Rail: 48px, `#1c1b1b`
- Canvas: `#0e0e0e` (deepest surface)
- Inspector: 320px slide-in from right, `#1c1b1b`

### Gates Master-Detail
- Table: 60% width, row hover `#2a2a2a`, selected has blue accent bar
- Detail: 40% width, `#1c1b1b`, stage timeline vertical

### Top Nav
- Text tabs, active has subtle blue underline (2px)
- No heavy tab bars or boxed navigation

---

## 6. Do's and Don'ts

### Do
- Use tonal shifts for all boundaries
- Use 4px left accent bars for type/status signaling
- Embrace the dark — let `#131313` breathe
- Use `0.25rem` (4px) radius everywhere
- Show status with 6px beads, not banners
- Use JetBrains Mono for all IDs, paths, timestamps

### Don't
- Use 1px solid borders for sectioning
- Use drop shadows (use ambient glow instead)
- Use ALL CAPS for labels
- Use 100% white text
- Show raw protocol URLs, file paths, or option counts
- Show the same value twice (dropdown + label below it)
- Use large bubbly border-radius

---

## 7. Stitch Reference Screens

Project: `projects/1858563870757498867`

| Screen | What it shows |
|--------|---------------|
| Agents Landing | Sidebar + empty state with starter chips |
| Active Chat | Messages, tool calls, chat input |
| Pipelines | Template cards with accent bars, prompt input |
| Pipeline Canvas | DAG nodes with inspector panel |
| Pipeline Flyout | Session list overlay on canvas |
| Gates | Master-detail task board |
| Gate Detail | Selected task + audit timeline |
| Workflows | 3-column browser with step timeline |
| Graph | Constellation view + side panel |
| Settings (×4) | Connection, Agents, MCP servers, Preferences |
| Command Palette | ⌘K overlay |
| Add Node Dialog | Type selector + form |
| Graft/Join Dialogs | Pipeline manipulation modals |
| Promotion Dialog | Memory write with search |
| Auth Dialog | Server sign-in confirmation |
| Traffic Monitor | Protocol traffic panel |
