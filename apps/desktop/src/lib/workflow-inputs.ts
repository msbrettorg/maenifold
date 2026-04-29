export type WorkflowInputType = 'text' | 'textarea' | 'number' | 'select' | 'boolean';

export interface WorkflowInputField {
  name: string;
  label: string;
  type: WorkflowInputType;
  required: boolean;
  description?: string;
  options: string[];
  defaultValue: string;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === 'object' && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function normalizeType(value: unknown, options: string[]): WorkflowInputType {
  const type = typeof value === 'string' ? value.toLowerCase() : 'text';
  if (options.length > 0 || type === 'select' || type === 'enum') return 'select';
  if (type === 'textarea' || type === 'multiline') return 'textarea';
  if (type === 'number' || type === 'integer') return 'number';
  if (type === 'boolean' || type === 'bool') return 'boolean';
  return 'text';
}

function normalizeOptions(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((entry): entry is string => typeof entry === 'string')
    : [];
}

function fieldFromRecord(name: string, value: unknown): WorkflowInputField | null {
  const record = asRecord(value);
  if (!record && typeof value !== 'string') return null;
  const source = record ?? { type: value };
  const options = normalizeOptions(source.options ?? source.enum);
  const label = typeof source.label === 'string' && source.label.trim()
    ? source.label
    : name.replace(/[-_]+/g, ' ').replace(/^./, (char) => char.toUpperCase());
  const defaultValue = source.default === undefined || source.default === null
    ? ''
    : String(source.default);
  return {
    name,
    label,
    type: normalizeType(source.type, options),
    required: source.required === true,
    description: typeof source.description === 'string' ? source.description : undefined,
    options,
    defaultValue,
  };
}

export function normalizeWorkflowInputs(input: unknown): WorkflowInputField[] {
  if (Array.isArray(input)) {
    return input.flatMap((entry) => {
      const record = asRecord(entry);
      const name = record?.name ?? record?.id ?? record?.key;
      return typeof name === 'string' && name.trim()
        ? [fieldFromRecord(name, record)].filter((field): field is WorkflowInputField => field !== null)
        : [];
    });
  }

  const record = asRecord(input);
  if (!record) return [];
  return Object.entries(record)
    .map(([name, value]) => fieldFromRecord(name, value))
    .filter((field): field is WorkflowInputField => field !== null);
}

export function coerceWorkflowInputValue(field: WorkflowInputField, value: string): unknown {
  if (field.type === 'number') {
    if (value.trim() === '') return undefined;
    const n = Number(value);
    if (!Number.isFinite(n)) {
      throw new Error(`Invalid number for "${field.name}": ${value}`);
    }
    return n;
  }
  if (field.type === 'boolean') {
    if (value === 'true') return true;
    if (value === 'false') return false;
  }
  return value;
}
