export interface JsonSchemaNode {
  type?: string;
  title?: string;
  description?: string;
  minimum?: number;
  properties?: Record<string, JsonSchemaNode>;
  items?: JsonSchemaNode;
  additionalProperties?: boolean | JsonSchemaNode;
  oneOf?: unknown;
  anyOf?: unknown;
  allOf?: unknown;
  [key: string]: unknown;
}

export interface ToolTextContent {
  type?: string;
  text?: string;
  [key: string]: unknown;
}

export interface ToolCallTextResult {
  content?: ToolTextContent[];
  isError?: boolean;
  [key: string]: unknown;
}

export function isPlainObject(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

export function cloneJsonValue<T>(value: T): T {
  if (value === undefined) {
    return value;
  }

  if (typeof structuredClone === 'function') {
    return structuredClone(value);
  }

  return JSON.parse(JSON.stringify(value)) as T;
}

export function normalizeForComparison(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map((entry) => normalizeForComparison(entry));
  }

  if (!isPlainObject(value)) {
    return value;
  }

  return Object.keys(value)
    .sort()
    .reduce<Record<string, unknown>>((acc, key) => {
      acc[key] = normalizeForComparison(value[key]);
      return acc;
    }, {});
}

export function stableStringify(value: unknown): string {
  return JSON.stringify(normalizeForComparison(value)) ?? 'undefined';
}

export function getNestedValue(root: unknown, path: string[]): unknown {
  let current = root;

  for (const segment of path) {
    if (!isPlainObject(current)) {
      return undefined;
    }

    current = current[segment];
  }

  return current;
}

export function buildNestedPatch(path: string[], value: unknown): unknown {
  if (path.length === 0) {
    return cloneJsonValue(value);
  }

  return path.reduceRight<unknown>((acc, key) => ({ [key]: acc }), cloneJsonValue(value));
}

export function formatSchemaLabel(key: string): string {
  return key
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/\b\w/g, (match) => match.toUpperCase());
}

export function pathToId(path: string[]): string {
  return path.length > 0 ? path.join('.') : 'root';
}

export function toJsonText(value: unknown): string {
  return JSON.stringify(value ?? null, null, 2);
}

export function inferSchemaFromValue(value: unknown): JsonSchemaNode {
  if (Array.isArray(value)) {
    if (value.every((entry) => typeof entry === 'string')) {
      return { type: 'array', items: { type: 'string' } };
    }

    return { type: 'array' };
  }

  if (isPlainObject(value)) {
    return { type: 'object', additionalProperties: true };
  }

  if (typeof value === 'boolean') {
    return { type: 'boolean' };
  }

  if (typeof value === 'number') {
    return { type: Number.isInteger(value) ? 'integer' : 'number' };
  }

  if (typeof value === 'string') {
    return { type: 'string' };
  }

  return {};
}

export function resolveSchema(schema: JsonSchemaNode, _value: unknown): JsonSchemaNode {
  if (
    schema.type ||
    schema.properties ||
    schema.items ||
    schema.additionalProperties !== undefined ||
    schema.oneOf ||
    schema.anyOf ||
    schema.allOf
  ) {
    return schema;
  }

  return { ...schema, type: 'json' };
}

export function extractToolText(result: ToolCallTextResult): string {
  const text = result.content?.find((entry) => typeof entry.text === 'string')?.text;

  if (typeof text !== 'string') {
    throw new Error('MCP tool result did not include content[0].text');
  }

  if (result.isError) {
    throw new Error(text);
  }

  return text;
}
