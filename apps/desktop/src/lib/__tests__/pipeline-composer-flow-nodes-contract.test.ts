import fs from 'node:fs';
import path from 'node:path';

import * as Module from '@/lib/pipeline-composer-flow-nodes';
import { describe, expect, it } from 'vitest';

const LIB_DIR = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const SOURCE_PATH = path.join(LIB_DIR, 'pipeline-composer-flow-nodes.ts');
const EXPECTED_EXPORTS = [
  'loadPositions',
  'pipelineNodesToFlow',
  'savePositions',
  'toFlowNodeData',
  'toPipelineNode',
  'toPipelineNodeFromViewNode',
].sort();
const SOURCE_DECLARED_EXPORTS = [
  'FlowNodeData',
  'loadPositions',
  'pipelineNodesToFlow',
  'savePositions',
  'toFlowNodeData',
  'toPipelineNode',
  'toPipelineNodeFromViewNode',
].sort();

function loc(filePath: string): number {
  return fs.readFileSync(filePath, 'utf8').trimEnd().split('\n').length;
}

function sourceExports(source: string): string[] {
  return source
    .split('\n')
    .flatMap((line) => {
      const match = line.match(/^export\s+(?:type|function)\s+([A-Za-z0-9_]+)/);
      return match ? [match[1]] : [];
    })
    .sort();
}

describe('pipeline-composer-flow-nodes split contract', () => {
  it('pipeline-composer-flow-nodes.ts is ≤300 LOC', () => {
    expect(loc(SOURCE_PATH)).toBeLessThanOrEqual(300);
  });

  it('module exports the locked public surface', () => {
    const source = fs.readFileSync(SOURCE_PATH, 'utf8');

    expect(Object.keys(Module).sort()).toEqual(EXPECTED_EXPORTS);
    expect(sourceExports(source)).toEqual(SOURCE_DECLARED_EXPORTS);
  });

  it('every .ts file in lib matching pipeline-composer-flow-nodes-*.ts is ≤300 LOC', () => {
    const matchingFiles = fs
      .readdirSync(LIB_DIR)
      .filter((name) => /^pipeline-composer-flow-nodes(?:-.*)?\.ts$/.test(name));

    expect(matchingFiles.length).toBeGreaterThan(0);
    for (const name of matchingFiles) {
      expect(loc(path.join(LIB_DIR, name)), `${name} should be ≤300 LOC`).toBeLessThanOrEqual(300);
    }
  });
});
