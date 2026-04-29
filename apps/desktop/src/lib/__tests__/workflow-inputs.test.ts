import { describe, expect, it } from 'vitest';
import { coerceWorkflowInputValue, type WorkflowInputField } from '../workflow-inputs';

function field(overrides: Partial<WorkflowInputField> = {}): WorkflowInputField {
  return {
    name: 'amount',
    label: 'Amount',
    type: 'number',
    required: false,
    options: [],
    defaultValue: '',
    ...overrides,
  };
}

describe('workflow-inputs', () => {
  it('rejects non-finite number values', () => {
    expect(() => coerceWorkflowInputValue(field(), '3.14abc')).toThrow(
      'Invalid number for "amount": 3.14abc',
    );
  });
});
