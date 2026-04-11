import { describe, expect, it } from 'vitest';
import { parseGateTaskBodyContent } from '../gate-task-body';

describe('parseGateTaskBodyContent', () => {
  it('strips frontmatter and gate-managed history sections', () => {
    const content = `---
id: T-131.0
title: Example
---

## Problem

Real task body.

## Acceptance criteria

- Shows the body

## Pending

- Stage ID: pending
- Record Kind: outcome
- Agent: swe
`;

    expect(parseGateTaskBodyContent(content)).toBe(`## Problem

Real task body.

## Acceptance criteria

- Shows the body`);
  });

  it('keeps non-gate sections that do not contain canonical gate bullets', () => {
    const content = `## Notes

- Stage ID: mentioned in prose
- But not a managed section
`;

    expect(parseGateTaskBodyContent(content)).toBe(content.trimEnd());
  });

  it('returns an empty string when the file only contains frontmatter and gate history', () => {
    const content = `---
id: T-131.0
---

## Pending

- Stage ID: pending
- Record Kind: outcome
`;

    expect(parseGateTaskBodyContent(content)).toBe('');
  });
});
