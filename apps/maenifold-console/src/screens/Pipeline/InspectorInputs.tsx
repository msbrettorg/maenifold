import { PropertyTree } from "./PropertyTree";

import { type PipelineNode } from "@/lib/pipeline";

type InspectorInputsProps = {
  node: PipelineNode;
};

export function InspectorInputs({ node }: InspectorInputsProps) {
  return (
    <PropertyTree
      data={node.inputArtifact ?? node.toolArgs ?? node.inputSchema}
      emptyMessage="No structured inputs were recorded for this node."
    />
  );
}
