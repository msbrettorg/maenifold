import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

describe('ChatInputBar source contract', () => {
  it('does not reference built-in slash command merging', () => {
    const source = readFileSync(join(process.cwd(), 'src/components/ChatInputBar.vue'), 'utf8');

    expect(source).not.toContain('mergeWithBuiltInSlashCommands');
    expect(source).not.toContain('default-commands');
  });
});
