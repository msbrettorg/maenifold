import { onBeforeUnmount, type Ref } from 'vue';
import cytoscape, { type Core } from 'cytoscape';
import coseBilkent from 'cytoscape-cose-bilkent';
import type { ForceNode, ForceEdge } from '../lib/graph-types';

cytoscape.use(coseBilkent);

type HoverCallback = (node: ForceNode | null) => void;
type ClickCallback = (node: ForceNode) => void;
type DoubleClickCallback = (node: ForceNode) => void;

export function useForceGraph(
  containerRef: Ref<HTMLElement | null>,
  onHover?: HoverCallback,
  onClick?: ClickCallback,
  onDoubleClick?: DoubleClickCallback,
) {
  let cy: Core | null = null;
  let nodeMap = new Map<string, ForceNode>();

  function start(nodes: ForceNode[], edges: ForceEdge[]) {
    stop();
    const el = containerRef.value;
    if (!el) return;

    // mind.html aesthetic: monochrome white-on-deep-space; structure emerges from density
    const labelColor = 'rgba(255, 255, 255, 0.7)';
    const labelHighlightColor = 'rgb(255, 255, 255)';
    const labelDimmedColor = 'rgba(255, 255, 255, 0.25)';

    nodeMap = new Map();
    for (const n of nodes) nodeMap.set(n.id, n);

    const elements: cytoscape.ElementDefinition[] = [];

    for (const n of nodes) {
      elements.push({
        data: { id: n.id, label: n.label, nodeRadius: n.radius, nodeColor: n.color },
      });
    }

    const nodeIds = new Set(nodes.map((n) => n.id));
    const addedEdges = new Set<string>();
    for (const e of edges) {
      const src = String(typeof e.source === 'object' ? (e.source as ForceNode).id : e.source);
      const tgt = String(typeof e.target === 'object' ? (e.target as ForceNode).id : e.target);
      if (!nodeIds.has(src) || !nodeIds.has(tgt)) continue;
      const key = src < tgt ? `${src}-${tgt}` : `${tgt}-${src}`;
      if (addedEdges.has(key)) continue;
      addedEdges.add(key);
      elements.push({
        data: { source: src, target: tgt, weight: e.weight },
      });
    }

    cy = cytoscape({
      container: el,
      elements,
      style: [
        {
          selector: 'node',
          style: {
            'width': 'data(nodeRadius)',
            'height': 'data(nodeRadius)',
            'background-color': 'data(nodeColor)',
            'background-opacity': 0.95,
            'border-width': 0,
            'label': 'data(label)',
            'color': labelColor,
            'font-size': '8px',
            'font-family': '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace',
            'font-weight': 400,
            'text-valign': 'bottom',
            'text-halign': 'center',
            'text-margin-y': 4,
            'text-max-width': '80px',
            'text-wrap': 'ellipsis',
            // Hide labels until zoomed in — structure first, names second
            'min-zoomed-font-size': 14,
            'overlay-opacity': 0,
            'transition-property': 'border-width border-color background-opacity',
            'transition-duration': 150,
          } as cytoscape.Css.Node,
        },
        {
          selector: 'edge',
          style: {
            'width': 0.5,
            'line-color': 'rgba(255, 255, 255, 0.05)',
            'curve-style': 'haystack',
            'haystack-radius': 0.5,
            'opacity': 1,
            'overlay-opacity': 0,
          } as cytoscape.Css.Edge,
        },
        {
          selector: 'node:active',
          style: {
            'overlay-opacity': 0,
          } as cytoscape.Css.Node,
        },
        {
          selector: '.highlighted',
          style: {
            'background-color': labelHighlightColor,
            'background-opacity': 1,
            'border-width': 0,
            'color': labelHighlightColor,
            'font-size': '10px',
            'font-weight': 500,
            'z-index': 10,
          } as cytoscape.Css.Node,
        },
        {
          selector: '.selected',
          style: {
            'border-color': labelHighlightColor,
            'border-opacity': 0.9,
            'border-width': 2,
            'background-opacity': 1,
            'color': labelHighlightColor,
            'z-index': 20,
          } as cytoscape.Css.Node,
        },
        {
          selector: '.highlighted-edge',
          style: {
            'line-color': 'rgba(255, 255, 255, 0.45)',
            'width': 1,
            'opacity': 1,
            'z-index': 10,
          } as cytoscape.Css.Edge,
        },
        {
          selector: '.dimmed',
          style: {
            'background-opacity': 0.15,
            'opacity': 0.15,
            'color': labelDimmedColor,
          } as cytoscape.Css.Node,
        },
        {
          selector: '.dimmed-edge',
          style: {
            'opacity': 0.02,
          } as cytoscape.Css.Edge,
        },
        // LOD: hide labels at low zoom
        {
          selector: '.lod-no-labels',
          style: {
            'label': '',
          } as cytoscape.Css.Node,
        },
      ],
      layout: {
        name: 'cose-bilkent',
        quality: 'default',
        nodeDimensionsIncludeLabels: true,
        randomize: true,
        animate: 'end',
        animationDuration: 800,
        fit: true,
        padding: 50,
        nodeRepulsion: 8000,
        idealEdgeLength: 120,
        edgeElasticity: 0.1,
        nestingFactor: 0.1,
        gravity: 0.15,
        gravityRange: 1.5,
        numIter: 3000,
        tile: true,
      } as cytoscape.LayoutOptions,
      minZoom: 0.15,
      maxZoom: 5,
      wheelSensitivity: 0.3,
      pixelRatio: 'auto',
      // Performance: texture during interaction for 60fps panning
      textureOnViewport: true,
      hideEdgesOnViewport: false,
      hideLabelsOnViewport: true,
    });

    function selectNode(nodeId: string): void {
      cy?.nodes().removeClass('selected');
      cy?.getElementById(nodeId).addClass('selected');
    }

    // Dynamic LOD based on zoom level
    cy.on('zoom', () => {
      if (!cy) return;
      const zoom = cy.zoom();
      const nodes = cy.nodes();

      if (zoom < 0.4) {
        // Far out: hide all labels, edges nearly invisible — let density carry structure
        nodes.addClass('lod-no-labels');
        cy.edges().style('opacity', 0.6);
      } else if (zoom < 0.8) {
        // Medium: show labels on large nodes only
        nodes.forEach((n) => {
          if ((n.data('nodeRadius') ?? 0) >= 4) {
            n.removeClass('lod-no-labels');
          } else {
            n.addClass('lod-no-labels');
          }
        });
        cy.edges().style('opacity', 0.85);
      } else {
        // Close: full detail
        nodes.removeClass('lod-no-labels');
        cy.edges().style('opacity', 1);
      }
    });

    // Hover: highlight node + neighbors, dim everything else
    cy.on('mouseover', 'node', (e) => {
      const node = e.target;
      const neighborhood = node.neighborhood().add(node);
      cy!.elements().addClass('dimmed').edges().addClass('dimmed-edge');
      neighborhood.removeClass('dimmed').addClass('highlighted');
      neighborhood.connectedEdges().removeClass('dimmed-edge').addClass('highlighted-edge');
      onHover?.(nodeMap.get(node.id()) ?? null);
    });

    cy.on('mouseout', 'node', () => {
      cy!.elements().removeClass('dimmed highlighted dimmed-edge highlighted-edge');
      onHover?.(null);
    });

    cy.on('tap', 'node', (e) => {
      selectNode(e.target.id());
      const n = nodeMap.get(e.target.id());
      if (n) onClick?.(n);
    });

    cy.on('dbltap', 'node', (e) => {
      selectNode(e.target.id());
      const n = nodeMap.get(e.target.id());
      if (n) onDoubleClick?.(n);
    });
  }

  function stop() {
    cy?.destroy();
    cy = null;
    nodeMap.clear();
  }

  function resetView() {
    cy?.fit(undefined, 50);
  }

  onBeforeUnmount(() => stop());

  return { start, stop, resetView };
}
