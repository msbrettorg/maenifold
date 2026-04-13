import { createAjv, type JsonSchema, validate } from '@jsonforms/core';
import type { ErrorObject, ValidateFunction } from 'ajv';

export type FallbackReason = 'missing' | 'unsupported' | 'complex' | 'unsafe';

interface SchemaComplexity {
  arrayCount: number;
  combinatorCount: number;
  enumValueCount: number;
  maxDepth: number;
  objectCount: number;
  propertyCount: number;
  visitedNodeCount: number;
}

interface SchemaAssessment {
  fallbackReason: FallbackReason | null;
  formSchema: JsonSchema | null;
  validator: ValidateFunction | null;
}

const EMPTY_RECORD: Readonly<Record<string, unknown>> = Object.freeze({});
const LOCAL_SCHEMA_REF_PREFIX = '#';
const MAX_SCHEMA_SERIALIZED_LENGTH = 64_000;
const UNSUPPORTED_FORM_KEYWORDS = Object.freeze([
  'dependentRequired',
  'dependentSchemas',
  'patternProperties',
  'propertyNames',
  'unevaluatedItems',
  'unevaluatedProperties',
]);
const COMPLEX_SCHEMA_LIMITS = Object.freeze({
  arrayCount: 3,
  combinatorCount: 0,
  maxDepth: 4,
  objectCount: 7,
  propertyCount: 18,
  score: 28,
  visitedNodeCount: 80,
});

export const schemaFormAjv = createAjv({ allErrors: true, strict: false });

export function inspectFormSchema(schema: unknown): SchemaAssessment {
  if (schema == null) {
    return { fallbackReason: 'missing', formSchema: null, validator: null };
  }
  if (!isSchemaObject(schema)) {
    return { fallbackReason: 'unsupported', formSchema: null, validator: null };
  }
  const schemaObject = schema as Record<string, unknown>;

  if (hasExternalRef(schemaObject)) {
    return { fallbackReason: 'unsafe', formSchema: null, validator: null };
  }
  const validator = compileSchemaValidator(schemaObject);

  if (isSchemaTooLarge(schemaObject)) {
    return { fallbackReason: 'complex', formSchema: null, validator };
  }

  const complexity = analyzeSchema(schemaObject);
  const score =
    complexity.propertyCount
    + complexity.objectCount * 2
    + complexity.arrayCount * 4
    + complexity.combinatorCount * 6
    + Math.max(0, complexity.maxDepth - 2) * 4
    + Math.ceil(complexity.enumValueCount / 10);

  if (
    complexity.combinatorCount > COMPLEX_SCHEMA_LIMITS.combinatorCount
    || complexity.maxDepth > COMPLEX_SCHEMA_LIMITS.maxDepth
    || complexity.arrayCount > COMPLEX_SCHEMA_LIMITS.arrayCount
    || complexity.objectCount > COMPLEX_SCHEMA_LIMITS.objectCount
    || complexity.propertyCount > COMPLEX_SCHEMA_LIMITS.propertyCount
    || complexity.visitedNodeCount > COMPLEX_SCHEMA_LIMITS.visitedNodeCount
    || score > COMPLEX_SCHEMA_LIMITS.score
  ) {
    return { fallbackReason: 'complex', formSchema: null, validator };
  }

  if (!hasObjectShape(schemaObject) || hasUnsupportedFormFeatures(schemaObject) || validator == null) {
    return { fallbackReason: 'unsupported', formSchema: null, validator };
  }

  return {
    fallbackReason: null,
    formSchema: schema as JsonSchema,
    validator,
  };
}

export function validateFormData(
  validator: ValidateFunction | null,
  data: Record<string, unknown>,
): boolean {
  return (getFormValidationErrors(validator, data)?.length ?? -1) === 0;
}

export function getFormValidationErrors(
  validator: ValidateFunction | null,
  data: Record<string, unknown>,
): ErrorObject[] | null {
  return validator ? validate(validator, data) : null;
}

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function cloneRecord<T extends Record<string, unknown>>(value: T): T {
  if (typeof structuredClone === 'function') {
    try {
      return structuredClone(value);
    } catch {
      // Vue reactive proxies contain non-cloneable internal state.
    }
  }

  return JSON.parse(JSON.stringify(value)) as T;
}

export function stringifyRecord(value: Record<string, unknown>): string {
  return JSON.stringify(value, null, 2);
}

export function normalizeFormData(value: unknown): Record<string, unknown> {
  return isRecord(value) ? cloneRecord(value) : cloneRecord(EMPTY_RECORD);
}

function isSchemaObject(value: unknown): value is Record<string, unknown> {
  return isRecord(value);
}

function compileSchemaValidator(schema: Record<string, unknown>): ValidateFunction | null {
  try {
    return schemaFormAjv.compile(schema as JsonSchema);
  } catch {
    return null;
  }
}

function analyzeSchema(schema: Record<string, unknown>, depth = 1, seen = new Set<object>()): SchemaComplexity {
  if (seen.has(schema)) {
    return {
      arrayCount: 0,
      combinatorCount: 0,
      enumValueCount: 0,
      maxDepth: depth,
      objectCount: 0,
      propertyCount: 0,
      visitedNodeCount: 0,
    };
  }
  seen.add(schema);

  const properties = isSchemaObject(schema.properties) ? Object.values(schema.properties) : [];
  const combinators = [
    ...collectSchemaArray(schema.oneOf),
    ...collectSchemaArray(schema.anyOf),
    ...collectSchemaArray(schema.allOf),
  ];
  const childSchemas = collectChildSchemas(schema, properties, combinators);
  const stats: SchemaComplexity = {
    arrayCount: hasArrayShape(schema) ? 1 : 0,
    combinatorCount: combinators.length,
    enumValueCount: Array.isArray(schema.enum) ? schema.enum.length : 0,
    maxDepth: depth,
    objectCount: hasObjectShape(schema) ? 1 : 0,
    propertyCount: properties.length,
    visitedNodeCount: 1,
  };

  for (const child of childSchemas) {
    const childStats = analyzeSchema(child, depth + 1, seen);
    stats.arrayCount += childStats.arrayCount;
    stats.combinatorCount += childStats.combinatorCount;
    stats.enumValueCount += childStats.enumValueCount;
    stats.maxDepth = Math.max(stats.maxDepth, childStats.maxDepth);
    stats.objectCount += childStats.objectCount;
    stats.propertyCount += childStats.propertyCount;
    stats.visitedNodeCount += childStats.visitedNodeCount;
  }

  return stats;
}

function hasExternalRef(schema: Record<string, unknown>, seen = new Set<object>()): boolean {
  if (seen.has(schema)) return false;
  seen.add(schema);

  if (typeof schema.$ref === 'string' && schema.$ref.trim() && !schema.$ref.startsWith(LOCAL_SCHEMA_REF_PREFIX)) {
    return true;
  }

  const properties = isSchemaObject(schema.properties) ? Object.values(schema.properties) : [];
  const combinators = [
    ...collectSchemaArray(schema.oneOf),
    ...collectSchemaArray(schema.anyOf),
    ...collectSchemaArray(schema.allOf),
  ];

  return collectChildSchemas(schema, properties, combinators).some((child) => hasExternalRef(child, seen));
}

function hasUnsupportedFormFeatures(schema: Record<string, unknown>, seen = new Set<object>()): boolean {
  if (seen.has(schema)) return false;
  seen.add(schema);

  if (UNSUPPORTED_FORM_KEYWORDS.some((keyword) => schema[keyword] != null)) {
    return true;
  }

  if (isSchemaObject(schema.additionalProperties)) {
    return true;
  }

  const properties = isSchemaObject(schema.properties) ? Object.values(schema.properties) : [];
  const combinators = [
    ...collectSchemaArray(schema.oneOf),
    ...collectSchemaArray(schema.anyOf),
    ...collectSchemaArray(schema.allOf),
  ];

  return collectChildSchemas(schema, properties, combinators).some((child) => hasUnsupportedFormFeatures(child, seen));
}

function isSchemaTooLarge(schema: Record<string, unknown>): boolean {
  try {
    return JSON.stringify(schema).length > MAX_SCHEMA_SERIALIZED_LENGTH;
  } catch {
    return true;
  }
}

function hasObjectShape(schema: Record<string, unknown>): boolean {
  return schema.type === 'object' || isSchemaObject(schema.properties);
}

function hasArrayShape(schema: Record<string, unknown>): boolean {
  return schema.type === 'array' || schema.items != null || Array.isArray(schema.prefixItems);
}

function collectChildSchemas(
  schema: Record<string, unknown>,
  properties: unknown[],
  combinators: Record<string, unknown>[],
): Record<string, unknown>[] {
  return [
    ...properties.filter(isSchemaObject),
    ...combinators,
    ...collectSchemaArray(schema.prefixItems),
    ...collectSingleSchema(schema.items),
    ...collectSingleSchema(schema.additionalProperties),
    ...collectSingleSchema(schema.contains),
    ...collectSingleSchema(schema.if),
    ...collectSingleSchema(schema.then),
    ...collectSingleSchema(schema.else),
    ...collectSingleSchema(schema.not),
    ...collectSingleSchema(schema.propertyNames),
    ...collectSingleSchema(schema.unevaluatedItems),
    ...collectSingleSchema(schema.unevaluatedProperties),
    ...collectPropertySchemas(schema.$defs),
    ...collectPropertySchemas(schema.definitions),
    ...collectPropertySchemas(schema.patternProperties),
    ...collectPropertySchemas(schema.dependentSchemas),
  ];
}

function collectSchemaArray(value: unknown): Record<string, unknown>[] {
  return Array.isArray(value) ? value.filter(isSchemaObject) : [];
}

function collectSingleSchema(value: unknown): Record<string, unknown>[] {
  return isSchemaObject(value) ? [value] : [];
}

function collectPropertySchemas(value: unknown): Record<string, unknown>[] {
  return isSchemaObject(value) ? Object.values(value).filter(isSchemaObject) : [];
}
