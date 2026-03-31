---
id: graph-visualization
status: done
---

# graph-visualization

**Requirement**: FR-32
**Test Cases**: TC-32.11
**Component**: vendor/chrome-acp (frontend addition)

## Acceptance Criteria

1. The SPA includes a graph visualization panel alongside the chat panel (split-pane or tabbed layout)
2. Graph fetches data from `/api/graph/nodes` and `/api/graph/edges` on load
3. Nodes rendered as circles with labels; size proportional to degree (number of connections)
4. Edges rendered as lines with opacity proportional to co-occurrence weight
5. Force-directed layout using Sigma.js+Graphology (WebGL, ForceAtlas2) or Cytoscape.js (Canvas)
6. Nodes are colored by community (Louvain community ID → distinct hue)
7. Clicking a node fetches its ego-graph (`?depth=2&center=<name>`) and re-centers the view
8. Graph polls `GET /api/graph/nodes` every 30 seconds and re-renders if the response differs from the last fetch
9. The chrome-acp submodule SHALL be forked to `ma-collective/chrome-acp` — graph visualization is added to the fork, not upstream. The submodule URL points to the fork.

## Verdicts

- TC-32.11 (graph renders force-directed layout from API data): ADD — test does not exist yet
