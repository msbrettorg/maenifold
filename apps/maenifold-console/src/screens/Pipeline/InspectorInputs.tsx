import {
  JsonView,
  allExpanded,
  darkStyles,
  defaultStyles,
} from "react-json-view-lite";

import { Button } from "@/components/ui/button";
import {
  stringifyJson,
  type JsonValue,
  type PipelineNode,
} from "@/lib/pipeline";
import { type Theme } from "@/lib/theme";

type InspectorInputsProps = {
  node: PipelineNode;
  theme: Theme;
};

export function InspectorInputs({ node, theme }: InspectorInputsProps) {
  const data = (node.inputArtifact ?? node.toolArgs ?? node.inputSchema) as JsonValue | undefined;

  if (data === undefined) {
    return (
      <div className="flex h-full items-center justify-center rounded-lg border border-dashed border-border bg-muted/30 px-6 text-center text-sm text-muted-foreground">
        No structured inputs were recorded for this node.
      </div>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex items-center justify-end">
        <Button
          type="button"
          variant="ghost"
          size="xs"
          onClick={async () => {
            await navigator.clipboard.writeText(stringifyJson(data));
          }}
        >
          Copy JSON
        </Button>
      </div>
      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-border bg-card/60 p-4">
        <JsonView
          data={data}
          shouldExpandNode={allExpanded}
          style={theme === "dark" ? darkStyles : defaultStyles}
        />
      </div>
    </div>
  );
}
