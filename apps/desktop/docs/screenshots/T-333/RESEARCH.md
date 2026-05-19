# T-333.0 Knowledge tab redesign research

## Screenshot inventory and source note

All screenshots in this directory are 1920×1200 PNGs. Current maenifold screenshots were taken from the packaged Electron app launched with `scripts/uat-launch.sh T-333.0 9343`; the launch log contained `[uat] MAENIFOLD_UAT mode active`. Obsidian was present as `/Applications/Obsidian.app`, but its CLI support was disabled and the already-open app was a personal vault, so I did **not** capture that vault. Obsidian references are screenshots of official Obsidian Help pages. LogSeq was not installed locally; LogSeq references are screenshots of official `docs.logseq.com` pages.

Current maenifold screenshots:

- `current-vault-tree.png`
- `current-note-editor.png`
- `current-backlinks.png`
- `current-tags.png`
- `current-search.png`
- `current-graph.png`
- `current-graph-filtered.png`
- `current-daily-notes.png`
- `current-settings.png`

Reference screenshots:

- Obsidian: `obsidian-vault-tree.png`, `obsidian-note-editor.png`, `obsidian-backlinks.png`, `obsidian-tags.png`, `obsidian-search.png`, `obsidian-graph.png`, `obsidian-daily-notes.png`, `obsidian-settings.png`
- LogSeq: `logseq-vault-tree.png`, `logseq-note-editor.png`, `logseq-backlinks.png`, `logseq-tags.png`, `logseq-search.png`, `logseq-graph.png`, `logseq-daily-notes.png`, `logseq-settings.png`

## Sub-view analysis

### Vault/file browser

Maenifold has a vault tree with folders, file rows, note actions, sort order, and a search box (`current-vault-tree.png`). It is functional but visually dense: folder counts and note rows are exposed in one long scroll surface with little hierarchy, and off-screen rows were present in the accessibility tree even when not visibly reachable without scrolling. Obsidian's file explorer reference (`obsidian-vault-tree.png`) establishes the baseline of a dedicated file navigation plugin with folder-first affordances. LogSeq's contents/reference page (`logseq-vault-tree.png`) emphasizes page/block navigation rather than raw files.

### Note editor/reader

Maenifold opens a note into a central leaf with a tab, close button, heading, Edit button, and rendered Markdown (`current-note-editor.png`). The view is closer to read mode than an Obsidian-quality editor: there is no visible live preview/source toggle, outline, properties, aliases, tag chips, or inline wikilink autocomplete evidence. Obsidian's editor reference (`obsidian-note-editor.png`) and LogSeq's block editing reference (`logseq-note-editor.png`) show the expected editing models: Obsidian is page/file-centric Markdown; LogSeq is block-first with editable outlines.

### Backlinks / linked references

Maenifold has a right-side panel containing related concepts and notes that link to the selected note (`current-backlinks.png`). This is a partial backlink surface: it lists inbound note references, but it does not show linked/unlinked split, context snippets, backlink search/filtering, or block-level references. Obsidian's Backlinks plugin (`obsidian-backlinks.png`) documents a dedicated active-note backlink pane. LogSeq's linked references page (`logseq-backlinks.png`) documents the core LogSeq page model where references are block-level and contextual.

### Tags

No dedicated maenifold tag panel or tag index was visible in the Knowledge tab; the captured evidence is the opened-note view with no tag UI (`current-tags.png`). The note content can contain plain Markdown tags, but there is no visible tag index, tag search, nested tag display, or tag-driven navigation. Obsidian has a Tags view core plugin (`obsidian-tags.png`). LogSeq exposes tags as first-class pages/properties (`logseq-tags.png`).

### Search

Maenifold exposes a vault search box and a Tree/Search toggle (`current-search.png`). In the sampled run, the Search tab initially displayed "Search the vault to show matching notes"; after entering `graph`, the search remained sidebar-scoped and did not reveal advanced operators, result context, saved searches, search history, or tag/property query integration. Obsidian's Search plugin (`obsidian-search.png`) documents search terms/operators. LogSeq's search reference (`logseq-search.png`) shows search as part of a page/block database model.

### Daily notes

Maenifold has a `DailyLogs` folder with date-named Markdown files (`current-daily-notes.png`), but there is no visible "open today's daily note" command, daily-note setting, template binding, date format preference, or calendar affordance. Obsidian has a Daily notes core plugin (`obsidian-daily-notes.png`). LogSeq treats journals as a core navigation primitive (`logseq-daily-notes.png`), though the captured official docs page for `journals` was sparse.

### Settings/preferences

Maenifold Settings exposes MCP server JSON and Embeddings model management (`current-settings.png`). I did not find a dedicated Knowledge preferences page for vault location, daily note format, templates folder, graph forces, tag behavior, aliases, or import/export. Obsidian's Templates page (`obsidian-settings.png`) represents the class of per-plugin settings expected from core plugins. LogSeq's `config.edn` page (`logseq-settings.png`) represents graph-level configuration by file.

## Dedicated graph view analysis

The current maenifold graph is the highest-risk surface. The graph view evidence (`current-graph.png`, `current-graph-filtered.png`) shows a Cytoscape canvas with a left community list, top/side filter boxes, disabled community combobox, Reset/Reload buttons, and a right node inspector. The snapshot text reported 387 nodes and a very long list of communities. The visible canvas itself is visually underwhelming at full-window size: it has limited labeling/context, no obvious minimap or zoom controls, no visible local graph toggle, no grouping legend beyond the community list, and no exposed layout tuning. Interaction copy says "Drag to pan · Scroll to zoom · Click a node for details"; that is a basic interaction set, but it is far below Obsidian/LogSeq expectations for graph exploration.

Compared with Obsidian's graph reference (`obsidian-graph.png`), maenifold lacks an obvious distinction between global and local graph, group/filter controls with a mature visual model, graph display controls, and a graph UX optimized for note navigation rather than raw concept-community dump. Compared with LogSeq's graph reference (`logseq-graph.png`), maenifold lacks a page/block graph mental model, obvious page focus, and an integrated path from graph node to page context. Performance is unproven beyond the captured 387-node run; nothing in the UI demonstrates confidence at 10K nodes, progressive loading, level-of-detail rendering, or WebGL/canvas perf budgets.

Graph-specific comparison:

| Concern | maenifold-current | Obsidian reference | LogSeq reference | Citation |
|---|---|---|---|---|
| Node count handled smoothly | Partial: captured 387 nodes; no 10K evidence | Mature graph plugin; docs describe graph as relationship visualization | Page graph concept documented | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` |
| Layout algorithm | Partial/implicit Cytoscape canvas; no user control visible | User-facing graph plugin controls are documented | Page graph view is a first-class navigation view | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` |
| Filtering controls | Partial: community filter, graph filter, disabled community dropdown | Search/filter model documented | Query/page model can narrow graph context | `current-graph-filtered.png`, `obsidian-search.png`, `logseq-search.png` |
| Focus/local graph | Absent in captured UI | Obsidian has graph/local graph concepts in plugin ecosystem | LogSeq page graph focuses around page/block relationships | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` |
| Click/drag/zoom | Partial: helper copy says drag/pan/scroll/click | Expected baseline | Expected baseline | `current-graph.png` |
| Edge styling | Partial/unclear from screenshot; no legend/toggles visible | Relationships between notes are the core graph visual | Page/block references are graph edges | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` |

## Gap matrix

| Capability | maenifold-current | obsidian | logseq | screenshot-citation | notes |
|---|---|---|---|---|---|
| Vault/file browser | present | present | partial | `current-vault-tree.png`, `obsidian-vault-tree.png`, `logseq-vault-tree.png` | Maenifold has tree, counts, actions; needs cleaner hierarchy and vault semantics. |
| Markdown note rendering | present | present | partial | `current-note-editor.png`, `obsidian-note-editor.png`, `logseq-note-editor.png` | Maenifold renders Markdown but editor affordances are thin. |
| Live/source editor | partial | present | present | `current-note-editor.png`, `obsidian-note-editor.png`, `logseq-note-editor.png` | Maenifold has Edit button only; no visible split/live/source modes. |
| Wikilink autocomplete | absent | present | present | `current-note-editor.png`, `obsidian-note-editor.png`, `logseq-note-editor.png` | No visible autocomplete, alias, or unresolved-link affordance. |
| Backlinks panel | partial | present | present | `current-backlinks.png`, `obsidian-backlinks.png`, `logseq-backlinks.png` | Maenifold lists inbound links but lacks context/filtering/unlinked mentions. |
| Related concepts | present | partial | partial | `current-backlinks.png`, `obsidian-graph.png`, `logseq-graph.png` | Maenifold has similarity list; needs clearer separation from backlinks. |
| Tags index | absent | present | present | `current-tags.png`, `obsidian-tags.png`, `logseq-tags.png` | No dedicated tag browser visible. |
| Nested tags | absent | present | present | `current-tags.png`, `obsidian-tags.png`, `logseq-tags.png` | Required for Obsidian parity. |
| Full-text search | partial | present | present | `current-search.png`, `obsidian-search.png`, `logseq-search.png` | Sidebar search exists; advanced operators/results context absent. |
| Search operators | absent | present | partial | `current-search.png`, `obsidian-search.png`, `logseq-search.png` | Obsidian has explicit operator docs. |
| Semantic search blend | partial | absent/extension | partial | `current-backlinks.png`, `current-settings.png`, `logseq-search.png` | Embeddings exist; not integrated into search UX. |
| Daily notes | partial | present | present | `current-daily-notes.png`, `obsidian-daily-notes.png`, `logseq-daily-notes.png` | Maenifold has date files, not a daily-note product workflow. |
| Templates | absent | present | partial | `current-settings.png`, `obsidian-settings.png`, `logseq-settings.png` | No template folder/button/insert flow visible. |
| Aliases/properties | absent | present | present | `current-note-editor.png`, `obsidian-note-editor.png`, `logseq-settings.png` | No note properties surface visible. |
| Canvas/whiteboard | absent | present | absent/core unclear | `current-note-editor.png`, `obsidian-vault-tree.png`, `logseq-vault-tree.png` | No Obsidian Canvas equivalent in Knowledge tab. |
| Import/export | absent | partial | partial | `current-settings.png`, `obsidian-settings.png`, `logseq-settings.png` | No visible import/export affordance in captured Knowledge/Settings views. |
| Graph view | partial | present | present | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` | Maenifold has graph canvas but weak information architecture. |
| Graph filtering | partial | present | partial | `current-graph-filtered.png`, `obsidian-graph.png`, `logseq-graph.png` | Filters exist but are basic and one dropdown was disabled. |
| Local/focus graph | absent | present | present | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` | Needed for large graphs and note-centric navigation. |
| Graph layout controls | absent | present | partial | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` | No force/layout sliders or presets visible. |
| Node inspector | present | partial | partial | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` | Maenifold has inspector placeholder; needs richer content. |
| 10K-node graph perf | absent/unproven | partial | partial | `current-graph.png`, `obsidian-graph.png`, `logseq-graph.png` | Captured run only proves 387-node load; no perf controls or LOD. |
| Per-plugin Knowledge settings | absent | present | present | `current-settings.png`, `obsidian-settings.png`, `logseq-settings.png` | Current settings are MCP/embeddings, not Knowledge preferences. |

Gap count in this matrix: 24 capability rows.

## First-draft redesign proposal

Redesign Knowledge as a three-mode workspace: **Notes**, **Links**, and **Graph**. Notes should be Obsidian-compatible at the file model layer: vault root, folder tree, Markdown editor with source/live/preview modes, wikilink autocomplete, aliases/properties, tag chips, templates, daily-note command, and import/export. Links should separate exact backlinks, outgoing links, unlinked mentions, semantic related concepts, and block/context snippets so users can tell "linked by Markdown" apart from "similar by embedding."

Replace the graph UX with a note-first graph explorer rather than a raw concept-community dump. Keep Cytoscape only if it can deliver 10K-node interaction with level-of-detail, layout presets, local graph mode, filter chips, keyboard selection, node inspector, minimap/zoom controls, and persisted graph settings; otherwise select a graph engine in plan stage based on measured requirements. The graph should default to a focused local graph for the selected note and expose global graph as an intentional mode with aggressive filtering, community clustering, and progressive rendering.
