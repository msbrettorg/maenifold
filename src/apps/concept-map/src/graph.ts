// T-APP-001.4: D3 force-directed concept graph
import { select } from "d3-selection";
import {
  forceSimulation,
  forceLink,
  forceManyBody,
  forceCenter,
  forceCollide,
  type Simulation,
} from "d3-force";
import { drag } from "d3-drag";
import { zoom, zoomIdentity } from "d3-zoom";
import type { BuildContextResult, GraphNode, GraphEdge } from "./types";
import { getCommunityColor } from "./theme";

const NODE_RADIUS_DEFAULT = 8;
const NODE_RADIUS_CENTER = 16;
const NODE_RADIUS_EXPANDED = 10;

export interface Graph {
  mergeData(result: BuildContextResult): void;
  onNodeClick: ((conceptName: string) => void) | null;
}

function nodeRadius(n: GraphNode): number {
  if (n.isCenter) return NODE_RADIUS_CENTER;
  if (n.expanded) return NODE_RADIUS_EXPANDED;
  return NODE_RADIUS_DEFAULT;
}

export function createGraph(container: HTMLElement): Graph {
  const nodes = new Map<string, GraphNode>();
  const edgeMap = new Map<string, GraphEdge>();
  let onNodeClick: ((conceptName: string) => void) | null = null;

  const svg = select(container).append("svg");
  const root = svg.append("g").attr("class", "root");
  const edgeLayer = root.append("g").attr("class", "edges");
  const nodeLayer = root.append("g").attr("class", "nodes");

  const tooltip = document.getElementById("tooltip") as HTMLDivElement;

  const w = () => container.clientWidth;
  const h = () => container.clientHeight;

  // Zoom + pan
  const zoomHandler = zoom<SVGSVGElement, unknown>()
    .scaleExtent([0.1, 8])
    .on("zoom", (event) => {
      root.attr("transform", event.transform);
    });

  svg.call(zoomHandler);
  svg.call(zoomHandler.transform, zoomIdentity);

  // Force simulation
  const simulation: Simulation<GraphNode, GraphEdge> = forceSimulation<GraphNode, GraphEdge>()
    .force("link", forceLink<GraphNode, GraphEdge>().id((d) => d.id).distance((link) => {
      const edge = link as GraphEdge;
      const weight = typeof edge.weight === "number" ? edge.weight : 1;
      return Math.max(40, 120 / (weight + 0.1));
    }))
    .force("charge", forceManyBody().strength(-200))
    .force("center", forceCenter(w() / 2, h() / 2))
    .force("collide", forceCollide<GraphNode>().radius((d) => nodeRadius(d) + 4))
    .alphaDecay(0.02)
    .on("tick", ticked);

  function getEdgeNodes(): GraphNode[] {
    return Array.from(nodes.values());
  }

  function getEdges(): GraphEdge[] {
    return Array.from(edgeMap.values());
  }

  function ticked() {
    edgeLayer
      .selectAll<SVGLineElement, GraphEdge>("line.edge")
      .attr("x1", (d) => (d.source as GraphNode).x ?? 0)
      .attr("y1", (d) => (d.source as GraphNode).y ?? 0)
      .attr("x2", (d) => (d.target as GraphNode).x ?? 0)
      .attr("y2", (d) => (d.target as GraphNode).y ?? 0);

    nodeLayer
      .selectAll<SVGGElement, GraphNode>("g.node")
      .attr("transform", (d) => `translate(${d.x ?? 0},${d.y ?? 0})`);
  }

  function restartSimulation() {
    const ns = getEdgeNodes();
    const es = getEdges();

    simulation.nodes(ns);
    const linkForce = simulation.force<ReturnType<typeof forceLink<GraphNode, GraphEdge>>>("link");
    if (linkForce) linkForce.links(es);

    // Render edges
    const edgeSel = edgeLayer
      .selectAll<SVGLineElement, GraphEdge>("line.edge")
      .data(es, (d) => {
        const s = typeof d.source === "string" ? d.source : (d.source as GraphNode).id;
        const t = typeof d.target === "string" ? d.target : (d.target as GraphNode).id;
        return `${s}--${t}`;
      });

    edgeSel.exit().remove();

    edgeSel
      .enter()
      .append("line")
      .attr("class", "edge")
      .merge(edgeSel)
      .attr("stroke-width", (d) => Math.max(0.5, (d.weight ?? 1) * 2));

    // Render node groups
    const nodeSel = nodeLayer
      .selectAll<SVGGElement, GraphNode>("g.node")
      .data(ns, (d) => d.id);

    nodeSel.exit().remove();

    const nodeEnter = nodeSel
      .enter()
      .append("g")
      .attr("class", "node")
      .call(
        drag<SVGGElement, GraphNode>()
          .on("start", (event, d) => {
            if (!event.active) simulation.alphaTarget(0.3).restart();
            d.fx = d.x;
            d.fy = d.y;
          })
          .on("drag", (event, d) => {
            d.fx = event.x;
            d.fy = event.y;
          })
          .on("end", (event, d) => {
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null;
            d.fy = null;
          })
      )
      .on("click", (_event, d) => {
        if (onNodeClick) onNodeClick(d.id);
      })
      .on("mouseover", (event, d) => {
        // T-APP-001.8: Safe DOM construction — no innerHTML XSS
        tooltip.textContent = "";
        const strong = document.createElement("strong");
        strong.textContent = d.id;
        tooltip.appendChild(strong);
        tooltip.appendChild(document.createElement("br"));
        tooltip.appendChild(document.createTextNode(
          d.communityId !== null ? `Community: ${d.communityId}` : "No community"
        ));
        tooltip.appendChild(document.createElement("br"));
        tooltip.appendChild(document.createTextNode(
          `Score: ${d.weightedScore.toFixed(3)}`
        ));
        if (d.expanded) {
          tooltip.appendChild(document.createElement("br"));
          const em = document.createElement("em");
          em.textContent = "Expanded";
          tooltip.appendChild(em);
        }
        tooltip.style.display = "block";
        tooltip.style.left = `${(event.pageX as number) + 12}px`;
        tooltip.style.top = `${(event.pageY as number) - 12}px`;
      })
      .on("mousemove", (event) => {
        tooltip.style.left = `${(event.pageX as number) + 12}px`;
        tooltip.style.top = `${(event.pageY as number) - 12}px`;
      })
      .on("mouseout", () => {
        tooltip.style.display = "none";
      });

    // Circle
    nodeEnter.append("circle");

    // Center ring
    nodeEnter
      .filter((d) => d.isCenter)
      .append("circle")
      .attr("class", "center-ring")
      .attr("fill", "none")
      .attr("stroke-width", 2.5)
      .attr("stroke-opacity", 0.7);

    // Label
    nodeEnter
      .append("text")
      .attr("class", "node-label");

    const nodeMerged = nodeEnter.merge(nodeSel);

    // Update circles
    nodeMerged
      .select<SVGCircleElement>("circle:first-child")
      .attr("r", (d) => nodeRadius(d))
      .attr("fill", (d) => getCommunityColor(d.communityId))
      .attr("stroke", (d) => (d.expanded ? "#fff" : "none"))
      .attr("stroke-width", (d) => (d.expanded ? 2 : 0));

    // Update center ring radius
    nodeMerged
      .select<SVGCircleElement>("circle.center-ring")
      .attr("r", (d) => nodeRadius(d) + 4)
      .attr("stroke", (d) => getCommunityColor(d.communityId));

    // Update labels
    nodeMerged
      .select<SVGTextElement>("text.node-label")
      .attr("dy", (d) => nodeRadius(d) + 14)
      .text((d) => d.id);

    simulation.alpha(0.5).restart();
  }

  function edgeKey(a: string, b: string): string {
    return a < b ? `${a}--${b}` : `${b}--${a}`;
  }

  function mergeData(result: BuildContextResult): void {
    const centerName = result.conceptName;

    // Determine parent position for new node spawning
    const parentNode = nodes.get(centerName);
    const px = parentNode?.x ?? w() / 2;
    const py = parentNode?.y ?? h() / 2;

    // Upsert center node
    if (!nodes.has(centerName)) {
      nodes.set(centerName, {
        id: centerName,
        communityId: result.communityId,
        weightedScore: 1,
        expanded: false,
        isCenter: false,
        x: px,
        y: py,
      });
    }
    const center = nodes.get(centerName)!;
    center.communityId = result.communityId;
    center.expanded = true;

    // Mark as center if it's the first expansion and no other center exists
    const hasCenterNode = Array.from(nodes.values()).some((n) => n.isCenter);
    if (!hasCenterNode) {
      center.isCenter = true;
    }

    // Upsert related concept nodes
    for (const rel of result.directRelations) {
      const existing = nodes.get(rel.name);
      if (!existing) {
        // Spawn near parent
        const angle = Math.random() * 2 * Math.PI;
        const dist = 60 + Math.random() * 60;
        nodes.set(rel.name, {
          id: rel.name,
          communityId: rel.communityId,
          weightedScore: rel.weightedScore,
          expanded: false,
          isCenter: false,
          x: px + Math.cos(angle) * dist,
          y: py + Math.sin(angle) * dist,
        });
      } else {
        existing.communityId = rel.communityId;
        existing.weightedScore = rel.weightedScore;
      }

      // Upsert edge (deduplicated)
      const key = edgeKey(centerName, rel.name);
      if (!edgeMap.has(key)) {
        edgeMap.set(key, {
          source: centerName,
          target: rel.name,
          weight: rel.weightedScore,
        });
      } else {
        edgeMap.get(key)!.weight = rel.weightedScore;
      }
    }

    restartSimulation();
  }

  return {
    mergeData,
    get onNodeClick() {
      return onNodeClick;
    },
    set onNodeClick(cb) {
      onNodeClick = cb;
    },
  } as Graph;
}
