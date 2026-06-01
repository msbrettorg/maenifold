import { describe, expect, it } from 'vitest';

import {
  buildNestedPatch,
  extractToolText,
  formatSchemaLabel,
  resolveSchema,
} from './settingsSchema';

describe('settingsSchema helpers', () => {
  it('buildNestedPatch creates a nested object for a multi-segment path', () => {
    expect(buildNestedPatch(['decay', 'halfLifeDays'], 45)).toEqual({
      decay: {
        halfLifeDays: 45,
      },
    });
  });

  it('buildNestedPatch creates a single-level leaf object', () => {
    expect(buildNestedPatch(['debounceMs'], 200)).toEqual({
      debounceMs: 200,
    });
  });

  it('buildNestedPatch preserves null values at the leaf', () => {
    expect(buildNestedPatch(['a', 'b', 'c'], null)).toEqual({
      a: {
        b: {
          c: null,
        },
      },
    });
  });

  it('resolveSchema returns an explicitly typed schema unchanged', () => {
    const schema = { type: 'string' };

    expect(resolveSchema(schema, 'anything')).toBe(schema);
  });

  it('resolveSchema falls back to json for an unshaped schema', () => {
    expect(resolveSchema({}, 'anything')).toEqual({ type: 'json' });
  });

  it('resolveSchema returns a schema with properties unchanged', () => {
    const schema = { properties: { x: { type: 'string' } } };

    expect(resolveSchema(schema, 'anything')).toBe(schema);
  });

  it('resolveSchema returns a schema with oneOf unchanged', () => {
    const schema = {
      oneOf: [{ type: 'string' }, { type: 'number' }],
    };

    expect(resolveSchema(schema, 'anything')).toBe(schema);
  });

  it('extractToolText reads the first text payload from a content envelope', () => {
    expect(
      extractToolText({
        content: [{ type: 'text', text: '{"x":1}' }],
      })
    ).toBe('{"x":1}');
  });

  it('formatSchemaLabel humanizes camelCase keys', () => {
    expect(formatSchemaLabel('halfLifeDays')).toBe('Half Life Days');
  });
});
