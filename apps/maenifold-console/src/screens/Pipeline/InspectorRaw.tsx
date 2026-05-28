import { useMemo, useState } from "react";
import {
  JsonView,
  allExpanded,
  darkStyles,
  defaultStyles,
} from "react-json-view-lite";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  filterJsonValue,
  stringifyJson,
  type JsonValue,
} from "@/lib/pipeline";
import { type Theme } from "@/lib/theme";

type InspectorRawProps = {
  value: JsonValue;
  theme: Theme;
};

export function InspectorRaw({ value, theme }: InspectorRawProps) {
  const [query, setQuery] = useState("");
  const filteredValue = useMemo(
    () => filterJsonValue(value, query),
    [query, value],
  );

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-3">
        <Input
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="Search raw payload"
          className="max-w-xs"
        />
        <Button
          type="button"
          variant="ghost"
          size="xs"
          onClick={async () => {
            await navigator.clipboard.writeText(stringifyJson(value));
          }}
        >
          Copy JSON
        </Button>
      </div>

      <div className="min-h-0 flex-1 overflow-auto rounded-lg border border-border bg-card/60 p-4">
        {filteredValue ? (
          <JsonView
            data={filteredValue}
            shouldExpandNode={allExpanded}
            style={theme === "dark" ? darkStyles : defaultStyles}
          />
        ) : (
          <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
            No raw fields matched the current search.
          </div>
        )}
      </div>
    </div>
  );
}
