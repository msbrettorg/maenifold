# UI Mockups

Static HTML/PNG design references for the two operator-facing surfaces.

**These are design artifacts, not a frontend.** Per the constitution there is no in-tree frontend; clients reach maenifold through MCP. These files are reference renders produced by Stitch using off-the-shelf component compositions (Tailwind CSS) so that any future client (Electron, web, terminal) has a concrete visual target.

| Surface | Dark | Light | Source variant |
|---|---|---|---|
| Pipeline viewer (live DAG + log/session inspector) | `pipeline-viewer-dark.{html,png}` | `pipeline-viewer-light.{html,png}` | Stitch iter3 `pipeline-dark-v5c` / `pipeline-light-v5b` |
| Gate board (kanban + task detail + History/evidence timeline) | `gate-board-dark.{html,png}` | `gate-board-light.{html,png}` | Stitch iter3 `gate-dark-v5b` / `gate-light-v5` |

## Pipeline viewer

What the layout has to convey, in priority order:
1. Which node is **RUNNING right now** (the dark variant uses an orange ring on the active node — this is the live-state signal a pipeline viewer lives or dies on).
2. The DAG topology (left canvas, mini-map bottom-left).
3. The active node's tabs — Overview, **Logs**, I/O, Sessions, Raw — in a right-hand inspector.
4. Counters at the top: Active / Done / Failed / Elapsed.
5. Footer actions on the active node: Open session · Open workflow · Cancel node.

## Gate board

What the layout has to convey, in priority order:
1. The **active task with its History/evidence timeline** is the differentiator vs a generic kanban board — the dark variant dims the columns behind the task detail to focus attention on the timeline (PENDING / SWE / RED TEAM / BLUE TEAM stage chips, gpt-5.4 model, sprint 251, description, acceptance criteria).
2. Kanban columns by stage (TODO / IN FLIGHT / REMEDIATION / DONE in dark; PENDING / SWE / RED TEAM in light) — secondary navigation.
3. Top bar: process selector, sprint selector, search, priority filter, active agent indicator.
4. Footer actions: Mark blocked · Open in editor.

## Stack constraint

**Off-the-shelf components only.** No hand-rolled UI components. Tailwind CSS utility classes for layout; component library is consumer's choice (shadcn/ui, Material, etc.) as long as it composes the same surfaces above. Implementers MUST NOT hand-roll buttons, cards, tabs, kanban columns, mini-maps, etc.

## Source

Generated from Stitch MCP server (`maenifold-stitch`) during session `fc95c65e-52db-4fc7-b237-ec19fd40b14f`. Locked variants from iter3 were copied verbatim from `~/.copilot/session-state/.../files/stitch-iter3/` into this directory and renamed to surface-purpose filenames.
