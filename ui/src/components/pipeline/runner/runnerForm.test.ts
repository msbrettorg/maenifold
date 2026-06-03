import { describe, expect, it } from 'vitest';
import {
  buildInputs,
  formErrors,
  initialFormModel,
  isFormValid,
  missingRequired,
  parseInputSchema,
} from './runnerForm';

const SCHEMA = {
  type: 'object',
  required: ['title', 'count', 'mode'],
  properties: {
    title: { type: 'string', title: 'Title', description: 'A heading' },
    count: { type: 'integer' },
    enabled: { type: 'boolean' },
    mode: { type: 'string', enum: ['fast', 'slow'] },
    note: { type: 'string' },
  },
};

describe('runnerForm.parseInputSchema', () => {
  it('returns [] for non-object / missing-properties schemas', () => {
    expect(parseInputSchema(null)).toEqual([]);
    expect(parseInputSchema(undefined)).toEqual([]);
    expect(parseInputSchema({})).toEqual([]);
    expect(parseInputSchema({ properties: 'nope' } as unknown as Record<string, unknown>)).toEqual([]);
  });

  it('maps types, enum, required, title and description', () => {
    const fields = parseInputSchema(SCHEMA);
    const byKey = Object.fromEntries(fields.map((f) => [f.key, f]));

    expect(byKey.title.type).toBe('string');
    expect(byKey.title.label).toBe('Title');
    expect(byKey.title.description).toBe('A heading');
    expect(byKey.title.required).toBe(true);

    expect(byKey.count.type).toBe('number'); // integer -> number
    expect(byKey.count.required).toBe(true);

    expect(byKey.enabled.type).toBe('boolean');
    expect(byKey.enabled.required).toBe(false);

    expect(byKey.mode.type).toBe('enum');
    expect(byKey.mode.enumValues).toEqual(['fast', 'slow']);
    expect(byKey.mode.required).toBe(true);

    expect(byKey.note.required).toBe(false);
  });
});

describe('runnerForm.initialFormModel', () => {
  it('seeds booleans false and everything else empty string', () => {
    const model = initialFormModel(parseInputSchema(SCHEMA));
    expect(model.title).toBe('');
    expect(model.count).toBe('');
    expect(model.enabled).toBe(false);
    expect(model.mode).toBe('');
    expect(model.note).toBe('');
  });
});

describe('runnerForm.validation', () => {
  it('reports missing required fields (booleans excluded)', () => {
    const fields = parseInputSchema(SCHEMA);
    const model = initialFormModel(fields);
    expect(missingRequired(fields, model).sort()).toEqual(['count', 'mode', 'title']);
    expect(isFormValid(fields, model)).toBe(false);
  });

  it('passes once required fields are filled', () => {
    const fields = parseInputSchema(SCHEMA);
    const model = { ...initialFormModel(fields), title: 'Hi', count: '3', mode: 'fast' };
    expect(missingRequired(fields, model)).toEqual([]);
    expect(isFormValid(fields, model)).toBe(true);
    expect(formErrors(fields, model)).toEqual({});
  });

  it('flags a non-numeric number field', () => {
    const fields = parseInputSchema(SCHEMA);
    const model = { ...initialFormModel(fields), title: 'Hi', count: 'abc', mode: 'fast' };
    expect(formErrors(fields, model).count).toBe('Must be a number.');
    expect(isFormValid(fields, model)).toBe(false);
  });

  it('flags an out-of-enum value', () => {
    const fields = parseInputSchema(SCHEMA);
    const model = { ...initialFormModel(fields), title: 'Hi', count: '3', mode: 'bogus' };
    expect(formErrors(fields, model).mode).toBe('Invalid option.');
  });
});

describe('runnerForm.buildInputs', () => {
  it('coerces numbers/booleans and omits empty optionals', () => {
    const fields = parseInputSchema(SCHEMA);
    const model = {
      ...initialFormModel(fields),
      title: '  hello  ',
      count: '42',
      enabled: true,
      mode: 'slow',
      note: '',
    };
    expect(buildInputs(fields, model)).toEqual({
      title: 'hello',
      count: 42,
      enabled: true,
      mode: 'slow',
    });
  });

  it('always includes booleans even when false', () => {
    const fields = parseInputSchema({
      type: 'object',
      properties: { flag: { type: 'boolean' } },
    });
    expect(buildInputs(fields, { flag: false })).toEqual({ flag: false });
  });
});
