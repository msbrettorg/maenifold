#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const APP_ROOT = process.cwd();
const REPO_ROOT = resolveRepoRoot();
const TARGET_EXTENSIONS = new Set(['.vue', '.ts', '.tsx', '.css']);
const ALLOWLIST = new Set([
  'src/assets/tailwind.css',
  'components.json',
]);

const HEX_LITERAL_RE = /#[0-9a-fA-F]{3,8}\b/g;
const BORDER_DECLARATION_RE = /\bborder(?:-color|-style)?\s*:/i;

const mode = process.argv.includes('--all') ? 'all' : 'changed';
const baseRef = resolveBaseRef();

const candidates = mode === 'all'
  ? collectTrackedFiles()
  : collectChangedLines(baseRef);

const violations = [];

for (const [relativeFile, changedLines] of candidates.entries()) {
  if (ALLOWLIST.has(relativeFile)) {
    continue;
  }

  const extension = path.extname(relativeFile);
  if (!TARGET_EXTENSIONS.has(extension)) {
    continue;
  }

  const absoluteFile = path.join(APP_ROOT, relativeFile);
  const source = readFileSafe(absoluteFile);
  if (source == null) {
    continue;
  }

  const lines = source.split(/\r?\n/);
  const styleRanges = extension === '.vue' ? findVueStyleRanges(lines) : [];

  if (mode === 'all') {
    for (let index = 0; index < lines.length; index += 1) {
      lintLine(relativeFile, extension, lines[index] ?? '', index + 1, styleRanges, violations);
    }
    continue;
  }

  for (const lineNumber of changedLines) {
    const line = lines[lineNumber - 1] ?? '';
    lintLine(relativeFile, extension, line, lineNumber, styleRanges, violations);
  }
}

if (violations.length > 0) {
  console.error('\nDesign lint violations found:\n');
  for (const violation of violations) {
    console.error(`${violation.file}:${violation.line}  ${violation.rule}  ${violation.message}`);
  }
  console.error('\nUse design tokens (CSS vars / Tailwind utilities) instead of raw literals.');
  process.exit(1);
}

const scanned = mode === 'all'
  ? `${candidates.size} files`
  : `${candidates.size} changed files against ${baseRef}`;
console.log(`✅ lint:design passed (${scanned}).`);

function lintLine(file, extension, line, lineNumber, styleRanges, output) {
  if (HEX_LITERAL_RE.test(line)) {
    output.push({
      file,
      line: lineNumber,
      rule: 'no-hex-literals',
      message: 'Hex color literal is not allowed. Use CSS variables or Tailwind design tokens.',
    });
  }
  HEX_LITERAL_RE.lastIndex = 0;

  if (
    extension === '.vue'
    && isInRanges(lineNumber, styleRanges)
    && BORDER_DECLARATION_RE.test(line)
  ) {
    output.push({
      file,
      line: lineNumber,
      rule: 'no-component-border-declarations',
      message: 'Component-local border declarations are not allowed. Use tokenized Tailwind border utilities.',
    });
  }
}

function isInRanges(value, ranges) {
  return ranges.some((range) => value >= range.start && value <= range.end);
}

function findVueStyleRanges(lines) {
  const ranges = [];
  let start = null;

  for (let index = 0; index < lines.length; index += 1) {
    const lineNumber = index + 1;
    const line = lines[index] ?? '';
    if (start == null && /^\s*<style\b/i.test(line)) {
      start = lineNumber;
      continue;
    }

    if (start != null && /^\s*<\/style>/i.test(line)) {
      ranges.push({ start, end: lineNumber });
      start = null;
    }
  }

  if (start != null) {
    ranges.push({ start, end: lines.length });
  }

  return ranges;
}

function resolveRepoRoot() {
  const result = runGit(['rev-parse', '--show-toplevel']);
  if (!result) {
    throw new Error('lint-design must run inside a git repository.');
  }
  return result;
}

function resolveBaseRef() {
  const override = process.env.DESIGN_LINT_BASE?.trim();
  if (override) {
    return override;
  }

  if (process.env.GITHUB_BASE_REF) {
    const branchRef = `origin/${process.env.GITHUB_BASE_REF}`;
    if (runGit(['rev-parse', '--verify', '--quiet', branchRef])) {
      return branchRef;
    }
  }

  if (runGit(['rev-parse', '--verify', '--quiet', 'origin/main'])) {
    return 'origin/main';
  }

  return 'HEAD~1';
}

function collectTrackedFiles() {
  const entries = runGitLines(['ls-files', '--', '.']);
  const result = new Map();
  for (const entry of entries) {
    const relative = normalizeRelative(entry);
    if (relative) {
      result.set(relative, []);
    }
  }
  return result;
}

function collectChangedLines(baseRef) {
  const mergeBase = runGit(['merge-base', 'HEAD', baseRef]) ?? baseRef;
  const diffOutput = runGitRaw(['diff', '--no-color', '--unified=0', '--relative', '--diff-filter=ACMR', `${mergeBase}...HEAD`, '--', '.']);
  const map = new Map();
  if (!diffOutput) {
    return map;
  }

  let currentFile = null;
  let lineCursor = 0;

  for (const rawLine of diffOutput.split('\n')) {
    if (rawLine.startsWith('+++ b/')) {
      currentFile = normalizeRelative(rawLine.slice(6));
      if (currentFile && !map.has(currentFile)) {
        map.set(currentFile, new Set());
      }
      lineCursor = 0;
      continue;
    }

    if (rawLine.startsWith('@@')) {
      const match = /@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(rawLine);
      if (match) {
        lineCursor = Number.parseInt(match[1], 10) - 1;
      }
      continue;
    }

    if (!currentFile) {
      continue;
    }

    if (rawLine.startsWith('+') && !rawLine.startsWith('+++')) {
      lineCursor += 1;
      map.get(currentFile)?.add(lineCursor);
      continue;
    }

    if (rawLine.startsWith(' ') || rawLine === '') {
      lineCursor += 1;
    }
  }

  const normalized = new Map();
  for (const [file, lineSet] of map.entries()) {
    normalized.set(file, [...lineSet].sort((left, right) => left - right));
  }
  return normalized;
}

function normalizeRelative(value) {
  const resolved = path.isAbsolute(value)
    ? value
    : path.resolve(APP_ROOT, value);
  const relative = path.relative(APP_ROOT, resolved);
  if (relative.startsWith('..')) {
    return null;
  }
  return relative.split(path.sep).join('/');
}

function readFileSafe(filePath) {
  try {
    return readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}

function runGit(args) {
  const result = spawnSync('git', args, {
    cwd: APP_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  if (result.status !== 0) {
    return null;
  }
  const stdout = result.stdout.trim();
  return stdout.length > 0 ? stdout : null;
}

function runGitLines(args) {
  const output = runGit(args);
  if (!output) {
    return [];
  }
  return output.split('\n').map((line) => line.trim()).filter(Boolean);
}

function runGitRaw(args) {
  const result = spawnSync('git', args, {
    cwd: APP_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  if (result.status !== 0) {
    return '';
  }
  return result.stdout;
}
