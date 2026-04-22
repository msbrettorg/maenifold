#!/usr/bin/env node

/**
 * Component audit linter — enforces shadcn-vue usage and prevents hand-rolled UI.
 *
 * Rules:
 *   no-raw-button        <button> must be shadcn <Button>
 *   no-raw-input         <input> must be shadcn <Input>
 *   no-raw-select        <select> must be shadcn <Select>
 *   no-raw-textarea      <textarea> must be shadcn <Textarea>
 *   no-raw-table         <table> must be shadcn <Table>
 *   no-raw-progress      <progress> or role="progressbar" must be shadcn <Progress>
 *   no-raw-dialog        <dialog> or role="dialog" must be shadcn <Dialog>
 *   no-btn-classname     btn-* classnames bypass Button variant props
 *   no-sibling-css       .css files alongside components — inline or delete
 *   max-style-rules      <style> blocks with >15 CSS rules are too heavy
 *   max-loc              Components must be ≤300 lines
 *
 * Exemption: <!-- audit-ok: rule-name --> on the same line or the line above.
 *
 * Usage:
 *   node scripts/audit-components.mjs          # changed files only
 *   node scripts/audit-components.mjs --all    # all tracked files
 *
 * Exit code 0 = clean, 1 = violations found.
 */

import { readFileSync, readdirSync, statSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const APP_ROOT = process.cwd();
const COMPONENTS_DIR = path.join(APP_ROOT, 'src', 'components');
const UI_DIR = path.join(COMPONENTS_DIR, 'ui');
const TESTS_DIR = '__tests__';

const MAX_STYLE_RULES = 15;
const MAX_LOC = 300;

const RAW_ELEMENT_RULES = [
  { rule: 'no-raw-button', pattern: /<button[\s>]/i, message: 'Raw <button> — use shadcn <Button> from @/components/ui/button.' },
  { rule: 'no-raw-input', pattern: /<input[\s>]/i, message: 'Raw <input> — use shadcn <Input> from @/components/ui/input.' },
  { rule: 'no-raw-select', pattern: /<select[\s>]/i, message: 'Raw <select> — use shadcn <Select> from @/components/ui/select.' },
  { rule: 'no-raw-textarea', pattern: /<textarea[\s>]/i, message: 'Raw <textarea> — use shadcn <Textarea> from @/components/ui/textarea.' },
  { rule: 'no-raw-table', pattern: /<table[\s>]/i, message: 'Raw <table> — use shadcn Table from @/components/ui/table.' },
  { rule: 'no-raw-progress', pattern: /<progress[\s>]|role="progressbar"/i, message: 'Raw progress — use shadcn <Progress> from @/components/ui/progress.' },
  { rule: 'no-raw-dialog', pattern: /<dialog[\s>]|role="dialog"/i, message: 'Raw dialog — use shadcn <Dialog> from @/components/ui/dialog.' },
];

const BTN_CLASS_RE = /class="[^"]*\bbtn[-\s]/;

const mode = process.argv.includes('--all') ? 'all' : 'changed';
const violations = [];

// Collect .vue files to audit (skip ui/ and __tests__/)
const vueFiles = mode === 'all'
  ? collectAllVueFiles()
  : collectChangedVueFiles();

// Check for sibling .css files (always scan all — they're few)
for (const cssFile of collectSiblingCssFiles()) {
  violations.push({
    file: cssFile,
    line: 1,
    rule: 'no-sibling-css',
    message: 'Sibling .css file — inline into <style scoped> or delete. Component styles belong in the component.',
  });
}

// Lint each .vue file
for (const relPath of vueFiles) {
  const absPath = path.join(APP_ROOT, relPath);
  const source = readFileSafe(absPath);
  if (source == null) continue;

  const lines = source.split(/\r?\n/);

  // Rule: max-loc
  if (lines.length > MAX_LOC) {
    violations.push({
      file: relPath,
      line: lines.length,
      rule: 'max-loc',
      message: `${lines.length} lines exceeds ${MAX_LOC} limit — split this component.`,
    });
  }

  // Rule: max-style-rules
  const styleRanges = findVueStyleRanges(lines);
  for (const range of styleRanges) {
    let ruleCount = 0;
    for (let i = range.start - 1; i < range.end; i++) {
      if (/\{/.test(lines[i] ?? '')) ruleCount++;
    }
    // Subtract 1 for the <style> tag itself if it matched
    ruleCount = Math.max(0, ruleCount - 1);
    if (ruleCount > MAX_STYLE_RULES) {
      violations.push({
        file: relPath,
        line: range.start,
        rule: 'max-style-rules',
        message: `${ruleCount} CSS rules in <style> block (max ${MAX_STYLE_RULES}) — migrate to Tailwind utilities or shadcn primitives.`,
      });
    }
  }

  // Per-line rules
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    const lineNumber = i + 1;
    const prevLine = i > 0 ? (lines[i - 1] ?? '') : '';

    // Check exemptions
    const exemptionSource = line + ' ' + prevLine;

    // Raw HTML element rules
    for (const { rule, pattern, message } of RAW_ELEMENT_RULES) {
      if (pattern.test(line) && !isExempt(exemptionSource, rule)) {
        violations.push({ file: relPath, line: lineNumber, rule, message });
      }
    }

    // btn-* classname rule
    if (BTN_CLASS_RE.test(line) && !isExempt(exemptionSource, 'no-btn-classname')) {
      violations.push({
        file: relPath,
        line: lineNumber,
        rule: 'no-btn-classname',
        message: 'btn-* classname bypasses Button variant props — use variant/size props instead.',
      });
    }
  }
}

// Report
if (violations.length > 0) {
  console.error(`\n❌ Component audit: ${violations.length} violation(s) found:\n`);

  // Group by rule for readability
  const byRule = new Map();
  for (const v of violations) {
    if (!byRule.has(v.rule)) byRule.set(v.rule, []);
    byRule.get(v.rule).push(v);
  }

  for (const [rule, items] of byRule) {
    console.error(`  ${rule} (${items.length}):`);
    for (const v of items) {
      console.error(`    ${v.file}:${v.line}  ${v.message}`);
    }
    console.error('');
  }

  console.error('Exempt a line with: <!-- audit-ok: rule-name -->');
  console.error('Run with --all to scan every component.\n');
  process.exit(1);
}

const scanned = mode === 'all'
  ? `${vueFiles.length} components`
  : `${vueFiles.length} changed components`;
console.log(`✅ audit:components passed (${scanned}).`);

// --- helpers ---

function isExempt(text, rule) {
  return text.includes(`audit-ok: ${rule}`) || text.includes('audit-ok: *');
}

function findVueStyleRanges(lines) {
  const ranges = [];
  let start = null;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    if (start == null && /^\s*<style\b/i.test(line)) {
      start = i + 1;
      continue;
    }
    if (start != null && /^\s*<\/style>/i.test(line)) {
      ranges.push({ start, end: i + 1 });
      start = null;
    }
  }
  if (start != null) ranges.push({ start, end: lines.length });
  return ranges;
}

function collectAllVueFiles() {
  const results = [];
  walkDir(COMPONENTS_DIR, results);
  return results.map((f) => path.relative(APP_ROOT, f).split(path.sep).join('/'));
}

function walkDir(dir, results) {
  if (dir.startsWith(UI_DIR)) return;
  if (path.basename(dir) === TESTS_DIR) return;

  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkDir(full, results);
    } else if (entry.isFile() && entry.name.endsWith('.vue')) {
      results.push(full);
    }
  }
}

function collectChangedVueFiles() {
  const result = spawnSync('git', ['diff', '--name-only', '--diff-filter=ACMR', 'HEAD~1...HEAD', '--', 'src/components/'], {
    cwd: APP_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  if (result.status !== 0 || !result.stdout.trim()) return [];
  return result.stdout.trim().split('\n')
    .filter((f) => f.endsWith('.vue') && !f.includes('/ui/') && !f.includes('__tests__'));
}

function collectSiblingCssFiles() {
  const results = [];
  try {
    for (const entry of readdirSync(COMPONENTS_DIR, { withFileTypes: true })) {
      if (entry.isFile() && entry.name.endsWith('.css')) {
        results.push(`src/components/${entry.name}`);
      }
    }
  } catch { /* empty */ }
  return results;
}

function readFileSafe(filePath) {
  try {
    return readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}
