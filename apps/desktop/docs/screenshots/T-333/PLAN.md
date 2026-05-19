# T-333.0 Knowledge tab redesign plan

## Source inputs read

- Research report: `apps/desktop/docs/screenshots/T-333/RESEARCH.md`
- Screenshot inventory: 25 PNGs under `apps/desktop/docs/screenshots/T-333/`, verified at 1920×1200.
- Umbrella task: `todo/T-333.0-knowledge-tab-redesign-spec.md`
- Format references read: `todo/T-332.0-pipeline-typed-handoff-umbrella.md`, `todo/T-332.6-kill-handlebars-acp.md`, and `todo/T-332.7-edge-mapping-editor-v2.md`
- Missing requested reference: `todo/T-332.5-acp-input-schema.md` was not present in this worktree or repo-level `todo/` / `done/` search.

## Evidence baseline

The research report establishes that maenifold has a working Knowledge surface but not an Obsidian/LogSeq-quality product model:

- Vault/file browser exists but is dense and not clearly an Obsidian-compatible vault model: `current-vault-tree.png`, contrasted with `obsidian-vault-tree.png` and `logseq-vault-tree.png`.
- Editor is closer to rendered read mode than a first-class Markdown editor: `current-note-editor.png`, contrasted with `obsidian-note-editor.png` and `logseq-note-editor.png`.
- Backlinks exist only as a partial right rail and semantic concept list; they lack linked/unlinked split, context snippets, and filters: `current-backlinks.png`, contrasted with `obsidian-backlinks.png` and `logseq-backlinks.png`.
- Search is sidebar-scoped and lacks operators/result context/history/saved searches: `current-search.png`, contrasted with `obsidian-search.png` and `logseq-search.png`.
- Tags and properties are effectively absent from visible UI: `current-tags.png`, contrasted with `obsidian-tags.png` and `logseq-tags.png`.
- Daily notes exist only as files in `DailyLogs`, not as a workflow with open-today, template, format, or calendar affordances: `current-daily-notes.png`, contrasted with `obsidian-daily-notes.png` and `logseq-daily-notes.png`.
- Settings expose MCP and embeddings, not Knowledge preferences: `current-settings.png`, contrasted with `obsidian-settings.png` and `logseq-settings.png`.
- Graph is the highest-risk surface: `current-graph.png` shows a loading/empty state with `0 nodes`, while `current-graph-filtered.png` shows a single node after filtering and a long community list. The research notes a 387-node captured run but no proof for 5K/10K performance, local graph, layout controls, minimap/zoom controls, or mature filters. Obsidian documents graph settings, filters, groups, display, forces, time-lapse, and local graph in `obsidian-graph.png`; LogSeq documents page graph and linked/unlinked references in `logseq-graph.png`.

## Debate transcript

### Turn 1 — Workspace shape and graph priority

PROPOSER: The redesign should split Knowledge into Notes, Links, Search, Graph, and Settings. We should keep the existing two-tab `Memory` / `Graph` mental model only as a migration target for UI routing, not as product language. Research already shows the current surfaces are partial: `current-backlinks.png` mixes related concepts and linked-from, `current-search.png` leaves search as a sidebar tab, and `current-graph-filtered.png` turns the graph into a single oversized label after filtering. The plan should make Graph a first-class spec, not a subcomponent.

RUBBER-DUCK: That is too polite. The user called graph particularly egregious, and the screenshot backs it. `current-graph.png` says `0 nodes` while showing a big empty pane; `current-graph-filtered.png` has a single node with no edge, no local/global distinction, no settings panel, no force controls, no legend, no minimap, and no evidence of keyboard navigation. Obsidian's graph docs in `obsidian-graph.png` explicitly show Filters, Groups, Display, Forces, time-lapse, and Local Graph. LogSeq's `logseq-graph.png` ties graph to page context and linked/unlinked references. If the component tree merely says "improve graph," it fails. It needs performance budgets, engine choice, local/global modes, filter chips, and LOD.

PROPOSER: Agreed on specificity. Graph gets its own child spec with engine decision and subcomponents: local graph as default from selected note, global graph as intentional mode, filter chips, layout controls, community grouping, node inspector, and LOD. But we should not throw out Cytoscape automatically before comparing it to Sigma.js, D3-force, WebCola, and vis.js. Cytoscape gives mature graph semantics and plugins; Sigma.js gives WebGL scale; D3-force gives layout control but would force us to assemble accessibility, selection, hit testing, and graph affordances. The plan should pick an engine on architectural grounds now.

RUBBER-DUCK: Pick now, then. The current evidence says Cytoscape status quo is not earning trust. The requirement targets smooth pan/zoom at 5K and acceptable degrade at 10K. Cytoscape can render canvas/WebGL through extensions, but the product already uses it and still looks weak. D3-force alone is not a graph product; WebCola is mostly layout; vis.js has an aging ecosystem and bundle/UX tradeoffs. Sigma.js is designed for large graph rendering and WebGL LOD. If we keep Cytoscape, the plan sounds like rewarding the status quo despite the screenshot failure.

PROPOSER: The failure is product architecture and controls, not proven renderer incapability. But the target is 5K smooth / 10K degrade, and the graph must be note-navigation-first. I choose Sigma.js for rendering the new global graph and local graph canvas, with a worker-backed layout pipeline. Cytoscape can remain only as removed legacy surface, not the target. Layout computation can use graphology + force/FA2-style layout in a worker, while Sigma owns WebGL rendering, camera, labels, and LOD. That gives a better scale ceiling than D3-force/Cytoscape status quo while avoiding hand-building the renderer.

### Turn 2 — Vault file model and link store

PROPOSER: For vault model, adopt Obsidian-compatible Markdown files as the canonical file contract: YAML frontmatter between `---`, `aliases:`, `tags:`, and `cssclasses:`. The research calls out absent aliases/properties and tags (`current-note-editor.png`, `current-tags.png`). Obsidian compatibility is a sharper contract than inventing maenifold metadata and then maintaining an importer.

RUBBER-DUCK: Exact Obsidian conventions are not free. LogSeq is block-first and uses page properties differently; `logseq-note-editor.png` and `logseq-tags.png` establish pages/properties/tags as first-class. If maenifold blindly copies Obsidian, it may lose its memory graph identity. Also, `memory://AGENTS` in `current-backlinks.png` is not a normal file path. How do you reconcile memory URI notes, daily logs, and Markdown vault files?

PROPOSER: The user set Obsidian/LogSeq as quality bar, not as a demand to clone both data models. Obsidian's file model is the safer base because maenifold already has folders, Markdown rendering, and date-named files. We can support LogSeq-like blocks later via headings/block anchors, but the plan should not make block storage the core primitive. Memory URIs become resolved note identities over vault-backed Markdown files: every note has a stable ID, file path, frontmatter properties, outgoing wikilinks, tags, aliases, and optional memory URI aliases for current MCP compatibility.

RUBBER-DUCK: "Optional memory URI aliases" sounds like hidden compatibility. The constitution bans silent dual-mode behavior. You need a single canonical identity model. Also, backlink computation cannot live only in JS. If the renderer owns parsing links, tags, aliases, unlinked mentions, and semantic relatedness, freshness will drift across tabs and restarts. Obsidian feels instant because indexes update as the vault changes. LogSeq feels database-backed because references are core. Where does maenifold's store live?

PROPOSER: Canonical identity is vault note ID + normalized path. `memory://` becomes an imported/opened source reference displayed in properties, not a second identity. Link store lives in Rust-backed SQLite because it must survive restart, serve MCP domain state, and stay consistent outside a single renderer. The renderer can maintain optimistic in-memory projections, but the authoritative index is SQLite-backed: notes, aliases, tags, outgoing links, backlinks, unresolved links, unlinked mentions candidates, headings, blocks/snippets, and embedding document IDs. Freshness model: on save, enqueue parse/index immediately; on idle, run unlinked mention and embedding refresh; on demand, expose "index stale / refresh now" rather than silently masking stale results.

### Turn 3 — Daily notes, templates, and canvas

PROPOSER: Daily notes should be first-class core. Research shows `current-daily-notes.png` has a `DailyLogs` folder only, while `obsidian-daily-notes.png` documents a core plugin and LogSeq journals are a central navigation primitive in `logseq-daily-notes.png`. Daily notes are not a niche add-on; they are a knowledge-capture loop.

RUBBER-DUCK: Daily notes can become scope creep. The graph is broken, editor is thin, backlinks are partial, tags absent, search weak. Why spend a child spec on daily notes before the fundamentals?

PROPOSER: Because daily notes stress every fundamental: file creation, templates, date format settings, backlinks, tags, search, and graph edges. A daily note spec is small/medium if it depends on the vault/editor/index settings. It should cover open today, create if missing, date format, folder, template, previous/next day, and calendar affordance. It is not a separate product.

RUBBER-DUCK: Fine for daily notes. Canvas/whiteboard is harder. Obsidian has Canvas as a core plugin visible in its plugin list (`obsidian-vault-tree.png` / `obsidian-graph.png` sidebar). If "Obsidian is the bar," excluding canvas makes the spec incomplete. But including a full whiteboard could swallow the sprint and distract from note parity.

PROPOSER: Canvas is included as a lower-priority child spec focused on integration boundaries, not as a blocker for the vault/editor/links/graph MVP. The spec should define a canvas file format decision, node types, links to notes, image/file cards, and graph handoff, but it should not be bundled into the graph implementation spec. That prevents "canvas" from being a vague future wish while keeping graph and notes deliverable.

RUBBER-DUCK: Only if the execute stage writes the child spec with acceptance criteria. Otherwise it is just a parking lot. The decomposition must include a concrete `T-333.7-knowledge-canvas-whiteboard.md`, even if P2, with explicit dependencies and a "not required to pass graph/editor acceptance" boundary.

### Turn 4 — Search, semantic blend, and user trust

PROPOSER: Search should blend full-text and semantic search because maenifold already exposes embeddings in Settings (`current-settings.png`) and related concepts in the right rail (`current-backlinks.png`). We can exceed Obsidian by combining exact text search, tags/properties operators, backlinks, and semantic relatedness in one search workspace.

RUBBER-DUCK: That can confuse users. Obsidian's Search is clean because results match text/operator semantics (`obsidian-search.png`). If maenifold returns semantic hits in the same list as exact hits, users will not know why a result matched. LogSeq's search sits in a page/block database model, but it still separates linked/unlinked references. The current maenifold right rail already blurs "related concept" and "linked from"; copying that confusion into search would fail the bar.

PROPOSER: Then the decision is a two-lane search UX: default lane is exact full-text with operators, tag/property filters, snippets, and sort; optional semantic lane is visibly labeled "Semantic matches" with similarity score, explanation, and a toggle. Combined ranking is allowed only in an explicit "Blend" mode, with badges per result: Exact, Tag, Property, Link, Semantic. Search should never hide why a result appeared.

RUBBER-DUCK: Require testable copy and empty states. `current-search.png` says "Search the vault to show matching notes" even after a query according to the research, with no advanced operators or result context. The spec must define operator help, snippets, saved searches, and stale-index states. Semantic search should also respect privacy/trust: embeddings refresh must be visible and cancellable if it triggers inference or long-running indexing.

PROPOSER: Yes: search spec will require explicit index status, separated exact/semantic result groups, operator help, saved search persistence, and no invisible inference path. Embedding computation status belongs in Knowledge settings and search status.

### Turn 5 — Accessibility, testing, and execute-stage specs

PROPOSER: The execute stage should author 7 child specs: vault/editor, index/backlinks/tags, search, graph, daily notes/templates, settings/import-export, and canvas. Pipelines should vary: TDD for stores/indexing, SDD for UI specs, SLC for graph and canvas due cross-cutting UI/performance.

RUBBER-DUCK: You are still under-specifying accessibility and headless test story for graph engine. Cytoscape has headless test utilities and a mature plugin ecosystem. Sigma.js is WebGL-heavy, so headless tests can become brittle. If the plan chooses Sigma, the graph spec must include an accessibility companion: keyboard-selectable node list, details panel, search/filter controls outside the canvas, and deterministic unit tests for data transforms separate from visual rendering. Otherwise Sigma solves performance but worsens accessibility.

PROPOSER: Good objection. The graph spec will require the WebGL canvas to be progressive enhancement over an accessible control plane: node list, selected-node inspector, filter chips, layout settings, and keyboard operations all exposed as DOM controls. Headless tests validate graph data projection, filter application, LOD thresholds, layout job scheduling, and inspector content. CDP UAT validates pan/zoom/selection in the real packaged app. The plan chooses Sigma only with that split.

RUBBER-DUCK: Also specify rejected engines. D3-force gives layout quality and animation but no product graph shell; WebCola is layout, not renderer; vis.js is easy but less convincing for 10K and modern ecosystem; Cytoscape is familiar/headless-friendly but status quo does not demonstrate the scale/UX. The decision table needs all of that.

PROPOSER: Captured below.

## Decision summary table

| Question | decision | chosen-option | rejected-alternatives | rationale | open-risks |
|---|---|---|---|---|---|
| Graph engine choice | Replace the status-quo graph renderer for the redesigned Graph workspace. | Sigma.js + graphology for WebGL rendering, camera, labels, LOD, and graph model; worker-backed layout jobs. | Cytoscape.js status quo; D3-force-only; WebCola-only; vis.js. | Current graph evidence (`current-graph.png`, `current-graph-filtered.png`) does not prove mature UX or 5K/10K scale. Sigma.js targets large graph WebGL rendering. D3-force and WebCola are layout pieces, not full product graph shells. vis.js is simpler but less compelling for 10K and modern LOD. Cytoscape has ecosystem/headless strengths but current use has not delivered the required UX. | WebGL headless tests are weaker; accessibility must be DOM/control-plane first; bundle impact must be measured; layout quality depends on worker implementation and presets. |
| Vault file model | Adopt Obsidian-compatible Markdown vault conventions as canonical. | Markdown files with `---` YAML frontmatter, `aliases:`, `tags:`, `cssclasses:`, note path as canonical identity, and import of current `memory://` source reference into properties. | Invent maenifold-only metadata and write importers; LogSeq block-first storage as canonical; dual identity between file path and `memory://`. | Research shows absent aliases/properties/tags (`current-note-editor.png`, `current-tags.png`), while Obsidian provides a strong file interoperability baseline. A maenifold-only model would add importer burden and reduce trust. | Need exact schema for current memory notes; frontmatter parser edge cases; must avoid hidden dual-mode identity; LogSeq block-level parity remains limited unless block anchors are added later. |
| Backlink/forward-link store | Make link/index storage authoritative outside the renderer. | Rust-owned SQLite index exposed through MCP/domain APIs; renderer uses subscriptions/optimistic cache only. Freshness: on save for exact links/tags/properties; on idle for unlinked mentions and embeddings; on demand for explicit refresh. | Pure JS renderer index; process-local in-memory index only; compute everything on demand. | Backlinks/tags/search/graph must survive restarts and feed MCP. Current backlinks are partial (`current-backlinks.png`); Obsidian/LogSeq make references core (`obsidian-backlinks.png`, `logseq-backlinks.png`). | Index invalidation complexity; concurrent saves; stale index states must be visible; SQLite schema must not overfit first UI. |
| Daily notes | Make daily notes a first-class core Knowledge workflow. | Child spec for open/create today, folder, date format, template, previous/next navigation, calendar affordance, and graph/search/index integration. | Treat date-named files as enough; make daily notes an optional plugin only; exclude from redesign. | Current `DailyLogs` folder is not a product workflow (`current-daily-notes.png`). Obsidian Daily notes and LogSeq journals are core capture loops (`obsidian-daily-notes.png`, `logseq-daily-notes.png`). | Requires settings/templates first; date/timezone decisions; migration from existing `DailyLogs` names. |
| Canvas/whiteboard | Include as an explicit child spec but do not couple it to graph implementation acceptance. | `T-333.7-knowledge-canvas-whiteboard.md` defines canvas file format, note cards, media/cards, links, and graph handoff. | Bundle full canvas into graph spec; omit canvas from T-333 plan; treat as vague future work. | Obsidian Canvas is part of the bar, but full whiteboard implementation is distinct from fixing vault/editor/links/graph. A child spec prevents an untracked gap without making graph dependent on canvas. | Canvas may require separate renderer/library research; file format choice can conflict with Obsidian `.canvas`; acceptance must avoid placeholder UI. |
| Semantic search blend | Separate exact search and semantic search by default; allow explicit blend mode. | Search workspace with exact full-text/operators as default, a labeled semantic group with scores/explanations, result badges, saved searches, snippets, and visible index/embedding status. | Obsidian-style text-only search; single blended list with hidden ranking; semantic-only search. | Maenifold has embeddings (`current-settings.png`) and related concepts (`current-backlinks.png`), but current UI already blurs link vs semantic relationships. Obsidian search is clean (`obsidian-search.png`); maenifold can exceed it only if match reasons are explicit. | Ranking complexity; semantic explanations can be noisy; embedding refresh must not create invisible long-running inference; result grouping may need tuning. |

## Component tree / spec decomposition for execute stage

The execute stage should author these new Gate specs in `todo/`, following the T-332 frontmatter + Source / Scope / Acceptance / Forbidden / Verification / Pending style.

| filename | scope-1-line | pipeline | priority | est-complexity |
|---|---|---|---|---|
| `T-333.1-knowledge-vault-editor.md` | Define Obsidian-compatible vault, file tree, Markdown editor modes, properties/frontmatter, aliases, tags-in-editor, wikilink autocomplete, and note identity. | sdd | P1 | XL |
| `T-333.2-knowledge-link-index-backlinks-tags.md` | Define the SQLite-backed link/tag/property index, inbound/outbound links, unlinked mentions, snippets, tag index, and freshness model. | tdd | P1 | XL |
| `T-333.3-knowledge-search.md` | Define exact full-text/operator search, semantic lane, blend mode, result snippets/badges, saved searches, and index status UX. | tdd | P1 | L |
| `T-333.4-knowledge-graph.md` | Define Sigma.js graph workspace: local/global graph, filters, layout controls, LOD, performance budgets, accessibility control plane, and CDP UAT. | slc | P1 | XL |
| `T-333.5-knowledge-daily-notes-templates.md` | Define daily notes and templates: open/create today, folder/date format, template binding, previous/next/calendar, and vault integration. | sdd | P1 | M |
| `T-333.6-knowledge-settings-import-export.md` | Define Knowledge settings for vault root, templates, daily notes, graph preferences, index status, import/export, and embedding model visibility. | sdd | P1 | L |
| `T-333.7-knowledge-canvas-whiteboard.md` | Define canvas/whiteboard integration boundaries: canvas file model, note/media cards, links, graph handoff, and acceptance separate from graph. | slc | P2 | XL |

Dependency outline:

- `T-333.1` seeds the vault/editor contract.
- `T-333.2` depends on the canonical note identity from `T-333.1`.
- `T-333.3` and `T-333.4` depend on `T-333.2` index outputs.
- `T-333.5` depends on `T-333.1` and contributes settings requirements to `T-333.6`.
- `T-333.6` should land early enough to host graph/search/index/daily-note preferences but can be split internally by settings section.
- `T-333.7` depends on `T-333.1` for note identity and on `T-333.2` for link/index integration; it must not block `T-333.4` acceptance.

## Graph view sub-plan

### Engine decision

Use **Sigma.js + graphology**, not Cytoscape.js status quo, D3-force-only, WebCola-only, or vis.js. The target graph needs WebGL rendering and explicit LOD to satisfy smooth pan/zoom at 5K nodes and acceptable degrade at 10K nodes. Layout computation should run in a worker; the UI should never block on a full global layout.

### Graph spec component decomposition

The execute-stage graph spec (`T-333.4-knowledge-graph.md`) should decompose into these surfaces:

1. **Graph data adapter**
   - Input: authoritative SQLite/index API from `T-333.2`.
   - Output: graphology graph with note nodes, tag/property nodes when enabled, unresolved-link nodes when enabled, and weighted edges for wikilinks/backlinks/tag co-occurrence/semantic similarity.
   - Must preserve match reason and edge type for inspector and filters.

2. **Local graph mode**
   - Default when a note is selected.
   - Depth controls: 1-hop, 2-hop, custom max nodes.
   - Edge-type toggles: outgoing, backlinks, tags, aliases, unresolved, semantic.
   - Opens from editor/backlinks/search/graph node click.
   - Must show selected note context and path; LogSeq-like page focus is required because `logseq-graph.png` emphasizes page graph and linked references.

3. **Global graph mode**
   - Explicit opt-in tab/state, not the default blank canvas.
   - Starts clustered and filtered, with graph summary counts.
   - Requires a visible "show global graph" action if node count exceeds threshold.
   - Community list becomes filter chips/groups, not a long passive rail like `current-graph-filtered.png`.

4. **Filter chips and query controls**
   - Chips for tags, folders, edge types, property filters, orphan/unresolved toggles, semantic threshold, and date ranges.
   - Search box filters nodes and labels; result count updates immediately.
   - Active filters are visible and removable.
   - Must support saved filter presets through settings.

5. **Layout controls**
   - Presets: Local radial, Force atlas/global, Hierarchical folder/tag, Timeline/daily.
   - Controls: gravity, repulsion, edge weight influence, label density, cluster strength, reset layout.
   - Layout jobs run in a worker with progress/cancel state.
   - Headless tests cover layout option serialization and job scheduling; CDP covers visual behavior.

6. **Level-of-detail strategy**
   - At high zoom: labels for selected, hovered, and high-degree nodes.
   - Mid zoom: labels for selected path and top-ranked nodes only.
   - Low zoom: cluster hulls/aggregate nodes and no dense labels.
   - Global graph over threshold starts in clustered mode.
   - Edges fade/aggregate by zoom and filter state.

7. **Node inspector**
   - Replaces placeholder "No concept selected" state with note title/path, aliases, tags, inbound/outbound counts, linked snippets, semantic neighbors, and actions: open note, open local graph, copy link, reveal in vault.
   - Must distinguish exact links from semantic similarity.

8. **Navigation and controls**
   - Mouse: drag pan, wheel zoom, click select, double-click open, drag node only when layout unlocked.
   - Keyboard: focus node list, arrow through visible nodes, Enter open, Escape clear, plus/minus zoom.
   - Buttons: zoom in/out, zoom to fit, reset camera, center selected.
   - Minimap is desirable only if it passes accessibility and performance budgets; otherwise zoom-to-fit and node list are mandatory.

9. **Accessibility and headless test story**
   - WebGL canvas is not the only interface. DOM controls expose filters, selected node, counts, node list, and inspector.
   - Unit tests cover graph data projection, filters, LOD thresholds, inspector model, and layout job state.
   - CDP UAT covers visual pan/zoom/select/open in the packaged Electron app.

10. **Performance budget**
   - 5K nodes / realistic edge count: smooth pan/zoom target, no blocking main-thread layout, responsive filter updates through worker/batched projection.
   - 10K nodes: accepted degrade means clustered initial render, label suppression, edge aggregation, and visible progress/cancel for expensive layout/filter changes.
   - The graph spec must require a synthetic vault fixture or generated graph fixture for performance UAT, but must not generate UAT checklists from the UAT plan.

### Explicit graph non-goals

- Do not implement graph as a screenshot-level facelift over the current Cytoscape canvas.
- Do not make semantic edges visually indistinguishable from explicit Markdown links.
- Do not hide expensive graph layout/index work without visible status.
- Do not require canvas/whiteboard to pass graph acceptance; canvas has its own child spec.

## Execute-stage authoring guidance

Each child spec should include:

- Frontmatter: `id`, `title`, `pipeline`, `priority`, `severity`, `parent: T-333.0`, `sprint: 333`, `stage: pending`, `discovered_in`, `discovered_at`, and dependencies when needed.
- Source: cite T-333 research and exact screenshot filenames.
- Scope: list product behavior, data contracts, affected surfaces, and explicit ownership boundaries.
- Acceptance criteria: implementation-ready, with UI/CDP requirements where UI is touched.
- Forbidden actions: no placeholders, no hidden inference/indexing, no implementation drift outside spec, no untracked alternate identity model.
- Required verification: typecheck/build/tests/CDP UAT commands appropriate to the eventual implementation role.
- Pending record: gate-compatible initial pending record.

## Plan conclusion

The plan chooses a note-centric Obsidian-compatible vault model, Rust/SQLite authoritative link index, first-class daily notes/templates, explicitly separated exact and semantic search, and a Sigma.js/WebGL graph redesign with accessible DOM controls and a 5K/10K performance target. Canvas/whiteboard is captured as a concrete child spec so it is not lost, but graph/editor/index acceptance should not depend on canvas delivery.
