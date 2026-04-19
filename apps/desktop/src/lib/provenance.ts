export type ProvenanceKind = 'gate' | 'pipeline' | 'thinking' | 'knowledge';

export interface ProvenanceLink {
  kind: ProvenanceKind;
  id: string;
  label?: string;
  lineage?: Partial<Record<ProvenanceKind, string>>;
  parentWorkflowId?: string;
}

export const PROVENANCE_ORDER: ProvenanceKind[] = ['gate', 'pipeline', 'thinking', 'knowledge'];

const KIND_LABELS: Record<ProvenanceKind, string> = {
  gate: 'Gate',
  pipeline: 'Pipeline',
  thinking: 'Thinking',
  knowledge: 'Knowledge',
};

export function provenanceKindLabel(kind: ProvenanceKind): string {
  return KIND_LABELS[kind];
}

export function compactProvenanceId(value: string, head = 12, tail = 8): string {
  if (value.length <= head + tail + 1) {
    return value;
  }

  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}
