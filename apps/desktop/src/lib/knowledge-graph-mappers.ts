import type { CommunityGraph, ConceptGraph, ForceEdge, ForceNode } from './graph-types';

function readCssToken(token: string, fallback: string): string {
  if (typeof window === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  return value || fallback;
}

const PRIMARY_FALLBACK = 'rgb(255 255 255)';
const SECONDARY_FALLBACK = 'rgb(160 170 178)';

export function colorForCommunityIndex(index: number): string {
  const hue = (index * 137.508) % 360;
  return `hsl(${hue.toFixed(2)}, 40%, 75%)`;
}

export function radiusForDegree(degree: number, maxDegree: number): number {
  if (maxDegree <= 0) return 1.6;
  const ratio = Math.max(0, Math.min(1, degree / maxDegree));
  return 1.6 + 4.9 * Math.sqrt(ratio);
}

export function lumaForDegree(degree: number, maxDegree: number): string {
  if (maxDegree <= 0) return readCssToken('--color-text-secondary', SECONDARY_FALLBACK);
  const ratio = Math.max(0, Math.min(1, degree / maxDegree));
  const value = Math.round(160 + ratio * 80);
  const hex = value.toString(16).padStart(2, '0');
  return `#${hex}${hex}${hex}`;
}

export function mapCommunityGraph(graph: CommunityGraph | null): { nodes: ForceNode[]; edges: ForceEdge[] } {
  if (!graph) {
    return { nodes: [], edges: [] };
  }

  const maxSize = graph.nodes.reduce((max, n) => Math.max(max, n.size), 0);

  const nodes = graph.nodes.map((node, index) => ({
    id: `community-${node.id}`,
    label: node.anchor,
    radius: radiusForDegree(node.size, maxSize),
    color: colorForCommunityIndex(index),
  }));

  const known = new Set(nodes.map((node) => node.id));
  const edges = graph.edges
    .filter((edge) => known.has(`community-${edge.source}`) && known.has(`community-${edge.target}`))
    .map((edge) => ({
      source: `community-${edge.source}`,
      target: `community-${edge.target}`,
      weight: edge.weight,
    }));

  return { nodes, edges };
}

export function mapConceptGraph(graph: ConceptGraph | null): { nodes: ForceNode[]; edges: ForceEdge[] } {
  if (!graph) {
    return { nodes: [], edges: [] };
  }

  const maxOccurrences = graph.nodes.reduce((max, n) => Math.max(max, n.occurrences), 0);

  const nodes = graph.nodes.map((node) => ({
    id: node.name,
    label: node.name,
    radius: radiusForDegree(node.occurrences, maxOccurrences),
    color: node.name === graph.anchor
      ? readCssToken('--color-text-primary', PRIMARY_FALLBACK)
      : lumaForDegree(node.occurrences, maxOccurrences),
  }));

  const known = new Set(nodes.map((node) => node.id));
  const edges = graph.edges
    .filter((edge) => known.has(edge.source) && known.has(edge.target))
    .map((edge) => ({
      source: edge.source,
      target: edge.target,
      weight: edge.weight,
    }));

  return { nodes, edges };
}
