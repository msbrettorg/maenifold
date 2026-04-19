// Pure helpers and types extracted from SchemaForm.vue.
// Keeps SchemaForm.vue under the 300 LOC ceiling.
import { isRecord } from './schema-form';

export interface FieldSchema {
  type?: string | string[];
  enum?: unknown[];
  title?: string;
  description?: string;
  format?: string;
  minimum?: number;
  maximum?: number;
}

export interface SimpleObjectSchema extends Record<string, unknown> {
  type?: string | string[];
  properties: Record<string, FieldSchema>;
  required?: string[];
}

export type SimpleFieldKind =
  | 'string'
  | 'enum'
  | 'textarea'
  | 'boolean'
  | 'integer'
  | 'number';

export interface SimpleField {
  key: string;
  label: string;
  description?: string;
  kind: SimpleFieldKind;
  maximum?: number;
  minimum?: number;
  options?: Array<{ label: string; value: string }>;
  required: boolean;
}

export function getSchemaType(property: FieldSchema): string | null {
  if (typeof property.type === 'string') {
    return property.type;
  }
  if (Array.isArray(property.type)) {
    return property.type.find((entry): entry is string => typeof entry === 'string') ?? null;
  }
  return null;
}

export function resolveFieldKind(
  property: FieldSchema,
  propertyType: string | null,
): SimpleFieldKind | null {
  if (
    propertyType === 'string'
    && Array.isArray(property.enum)
    && property.enum.every((entry) => typeof entry === 'string')
  ) {
    return 'enum';
  }
  if (propertyType === 'string' && property.format === 'textarea') {
    return 'textarea';
  }
  if (propertyType === 'string') {
    return 'string';
  }
  if (propertyType === 'boolean') {
    return 'boolean';
  }
  if (propertyType === 'integer' || propertyType === 'number') {
    return propertyType;
  }
  return null;
}

export function toSimpleObjectSchema(schema: unknown): SimpleObjectSchema | null {
  if (!isRecord(schema) || !isRecord(schema.properties)) {
    return null;
  }

  const resolvedType = getSchemaType(schema as FieldSchema);
  if (resolvedType !== 'object') {
    return null;
  }

  const properties = Object.entries(schema.properties).reduce<Record<string, FieldSchema>>(
    (accumulator, [key, value]) => {
      if (!isRecord(value)) return accumulator;
      accumulator[key] = value as FieldSchema;
      return accumulator;
    },
    {},
  );

  if (Object.keys(properties).length === 0) return null;

  const allPropertiesSupported = Object.values(properties).every((property) => {
    const propertyType = getSchemaType(property);
    return resolveFieldKind(property, propertyType) !== null;
  });

  if (!allPropertiesSupported) return null;

  return {
    ...schema,
    properties,
    required: Array.isArray(schema.required)
      ? schema.required.filter((entry): entry is string => typeof entry === 'string')
      : [],
  } as SimpleObjectSchema;
}

export function buildSimpleFields(schema: SimpleObjectSchema | null): SimpleField[] {
  if (!schema) return [];
  const requiredFields = new Set(schema.required ?? []);
  return Object.entries(schema.properties).map(([key, property]) => {
    const resolvedType = getSchemaType(property);
    const kind = resolveFieldKind(property, resolvedType);
    const options = Array.isArray(property.enum)
      ? property.enum.map((value) => ({
          label: stringifyEnumLabel(value),
          value: String(value),
        }))
      : undefined;
    return {
      key,
      label: property.title?.trim() || sentenceCase(key),
      description:
        typeof property.description === 'string' && property.description.trim().length > 0
          ? property.description
          : undefined,
      kind: kind ?? 'string',
      maximum: typeof property.maximum === 'number' ? property.maximum : undefined,
      minimum: typeof property.minimum === 'number' ? property.minimum : undefined,
      options,
      required: requiredFields.has(key),
    };
  });
}

export function computeSimpleErrors(
  fields: SimpleField[],
  values: Record<string, unknown>,
): Record<string, string> {
  const errors: Record<string, string> = {};
  for (const field of fields) {
    const value = values[field.key];
    if (field.required && isBlankValue(value)) {
      errors[field.key] = `${field.label} is required.`;
      continue;
    }
    if (
      (field.kind === 'integer' || field.kind === 'number')
      && value !== undefined
      && value !== null
      && value !== ''
    ) {
      if (typeof value !== 'number' || !Number.isFinite(value)) {
        errors[field.key] = `${field.label} must be a valid number.`;
        continue;
      }
      if (field.kind === 'integer' && !Number.isInteger(value)) {
        errors[field.key] = `${field.label} must be a whole number.`;
        continue;
      }
      if (field.minimum !== undefined && value < field.minimum) {
        errors[field.key] = `${field.label} must be ${field.minimum} or greater.`;
        continue;
      }
      if (field.maximum !== undefined && value > field.maximum) {
        errors[field.key] = `${field.label} must be ${field.maximum} or less.`;
      }
    }
  }
  return errors;
}

export function normalizeSimpleFieldValue(field: SimpleField, value: unknown): unknown {
  if (field.kind === 'boolean') {
    return value === true;
  }
  if (field.kind === 'integer' || field.kind === 'number') {
    const raw = typeof value === 'number' ? String(value) : String(value ?? '').trim();
    if (!raw) return undefined;
    const parsed = Number(raw);
    if (!Number.isFinite(parsed)) return undefined;
    return field.kind === 'integer' ? Math.trunc(parsed) : parsed;
  }
  return typeof value === 'string' ? value : String(value ?? '');
}

export function stringifyEnumLabel(value: unknown): string {
  if (value === '__none__') return 'None';
  return typeof value === 'string' && value.trim().length > 0 ? value : String(value ?? '');
}

export function sentenceCase(value: string): string {
  return value
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[_-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^./, (character) => character.toUpperCase());
}

export function isBlankValue(value: unknown): boolean {
  return value == null || (typeof value === 'string' && value.trim().length === 0);
}

export function formatValidationError(error: { instancePath?: string; message?: string }): string {
  const location = error.instancePath?.trim() ? error.instancePath : '/';
  const message = error.message?.trim() || 'Schema validation failed.';
  return `${location}: ${message}`;
}
