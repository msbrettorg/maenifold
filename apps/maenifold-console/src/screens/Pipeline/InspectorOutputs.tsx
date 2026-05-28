import { PropertyTree } from "./PropertyTree";

import { type PipelineNode } from "@/lib/pipeline";

type InspectorOutputsProps = {
  node: PipelineNode;
};

export function InspectorOutputs({ node }: InspectorOutputsProps) {
  return (
    <PropertyTree
      data={node.outputArtifact ?? node.output ?? node.outputSchema}
      emptyMessage="No structured outputs were recorded for this node."
    />
  );
}
