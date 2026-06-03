// Pipeline runner — run-input schema form model (pure, DOM-free, unit-testable).
//
// A pipeline definition may carry an `inputSchema` (JSON-Schema object shape:
// `{ type:'object', properties:{...}, required:[...] }`). When present the runner
// renders a typed form, one field per property. These helpers parse the schema
// into a flat field list, seed a form model, validate it (required + basic type
// checks), and coerce the model back into a `PipelineInputs` map for
// `store.runPipeline(id, { inputs })`. Backend support for `definition.inputSchema`
// landed in T-600.9.

import type { PipelineInputs } from '../../../lib/pipeline-types';

export type FieldType = 'string' | 'number' | 'boolean' | 'enum';

// Form-model values are the raw values bound to inputs. Text/number/enum fields
// bind to a string (native inputs/selects emit strings); booleans bind to a
// boolean. Coercion to the schema's real type happens in `buildInputs`.
export type FormValue = string | boolean;
export type FormModel = Record<string, FormValue>;

export interface SchemaField {
  key: string;
  label: string;
  type: FieldType;
  required: boolean;
  description?: string;
  // Present only when `type === 'enum'`; the allowed string values.
  enumValues?: string[];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

// Parse a JSON-Schema-ish object into an ordered field list. Anything that is
// not a `{ properties: {...} }` object yields an empty list (the runner then
// treats run as a simple action with an optional prompt).
export function parseInputSchema(
  schema: Record<string, unknown> | null | undefined
): SchemaField[] {
  if (!isRecord(schema) || !isRecord(schema.properties)) return [];

  const required = Array.isArray(schema.required)
    ? schema.required.filter((entry): entry is string => typeof entry === 'string')
    : [];

  const fields: SchemaField[] = [];
  for (const [key, raw] of Object.entries(schema.properties)) {
    if (!isRecord(raw)) continue;

    const enumValues =
      Array.isArray(raw.enum) && raw.enum.length > 0
        ? raw.enum.map((entry) => String(entry))
        : undefined;

    const declared = typeof raw.type === 'string' ? raw.type : 'string';
    let type: FieldType;
    if (enumValues) type = 'enum';
    else if (declared === 'number' || declared === 'integer') type = 'number';
    else if (declared === 'boolean') type = 'boolean';
    else type = 'string';

    fields.push({
      key,
      label: typeof raw.title === 'string' ? raw.title : key,
      type,
      required: required.includes(key),
      description: typeof raw.description === 'string' ? raw.description : undefined,
      enumValues,
    });
  }
  return fields;
}

// Seed a form model: booleans start false, everything else starts empty. Enums
// start empty (unset) so a required enum forces an explicit selection.
export function initialFormModel(fields: SchemaField[]): FormModel {
  const model: FormModel = {};
  for (const field of fields) {
    model[field.key] = field.type === 'boolean' ? false : '';
  }
  return model;
}

// Per-field validation errors keyed by field key. Empty object => valid.
export function formErrors(
  fields: SchemaField[],
  model: FormModel
): Record<string, string> {
  const errors: Record<string, string> = {};
  for (const field of fields) {
    if (field.type === 'boolean') continue; // a checkbox always has a value
    const value = model[field.key];
    const text = typeof value === 'string' ? value.trim() : '';

    if (field.required && text === '') {
      errors[field.key] = 'Required.';
      continue;
    }
    if (text === '') continue; // optional + empty is fine
    if (field.type === 'number' && Number.isNaN(Number(text))) {
      errors[field.key] = 'Must be a number.';
    }
    if (field.type === 'enum' && field.enumValues && !field.enumValues.includes(text)) {
      errors[field.key] = 'Invalid option.';
    }
  }
  return errors;
}

export function missingRequired(fields: SchemaField[], model: FormModel): string[] {
  return fields
    .filter((field) => {
      if (!field.required || field.type === 'boolean') return false;
      const value = model[field.key];
      return typeof value === 'string' ? value.trim() === '' : false;
    })
    .map((field) => field.key);
}

export function isFormValid(fields: SchemaField[], model: FormModel): boolean {
  return Object.keys(formErrors(fields, model)).length === 0;
}

// Coerce the form model into the `PipelineInputs` wire shape. Booleans pass
// through; numbers are parsed; empty optional fields are omitted entirely so the
// backend sees only the inputs the operator actually supplied.
export function buildInputs(fields: SchemaField[], model: FormModel): PipelineInputs {
  const inputs: PipelineInputs = {};
  for (const field of fields) {
    const value = model[field.key];
    if (field.type === 'boolean') {
      inputs[field.key] = Boolean(value);
      continue;
    }
    const text = typeof value === 'string' ? value.trim() : '';
    if (text === '') continue;
    inputs[field.key] = field.type === 'number' ? Number(text) : text;
  }
  return inputs;
}
