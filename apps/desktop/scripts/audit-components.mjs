#!/usr/bin/env node

/**
 * Component audit linter — enforces DESIGN.md's shadcn-vue/Tailwind contract.
 *
 * Rules:
 *   no-style-block       Vue SFCs must not contain <style> blocks
 *   no-raw-*             HTML controls must use shadcn-vue primitives
 *   no-dynamic-raw-*     <component :is="'input'"> raw-control bypasses are banned
 *   no-btn-classname     btn-* classnames bypass Button variant props
 *   no-sibling-css       .css files alongside components are banned
 *   max-loc              Components must be ≤300 lines
 *
 * audit-ok comments are banned. Any occurrence under src/ fails the audit.
 */

import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { countLinesForMaxLoc } from './audit-loc.mjs';

const APP_ROOT = process.cwd();
const COMPONENTS_DIR = path.join(APP_ROOT, 'src', 'components');
const UI_DIR = path.join(COMPONENTS_DIR, 'ui');
const SKIP_DIRS = new Set(['__tests__', 'test', 'tests', 'e2e', 'node_modules']);
const MAX_LOC = 300;

// DESIGN.md says no <style> blocks. Keep this empty unless PM approves a named migration exception.
const STYLE_EXCEPTIONS = new Set([]);

const RAW_CONTROL_EXCEPTIONS = new Map([]);

const RAW_CONTROL_RULES = [
  ['button', 'no-raw-button', 'Raw <button> — use shadcn <Button> from @/components/ui/button.'],
  ['input', 'no-raw-input', 'Raw <input> — use shadcn <Input> from @/components/ui/input.'],
  ['select', 'no-raw-select', 'Raw <select> — use shadcn <Select> from @/components/ui/select.'],
  ['textarea', 'no-raw-textarea', 'Raw <textarea> — use shadcn <Textarea> from @/components/ui/textarea.'],
  ['table', 'no-raw-table', 'Raw <table> — use shadcn Table from @/components/ui/table.'],
  ['progress', 'no-raw-progress', 'Raw <progress> — use shadcn <Progress> from @/components/ui/progress.'],
  ['dialog', 'no-raw-dialog', 'Raw <dialog> — use shadcn <Dialog> from @/components/ui/dialog.'],
];
const RAW_CONTROL_BY_TAG = new Map(RAW_CONTROL_RULES.map(([tag, rule, message]) => [tag, { rule, message }]));
const ROLE_RULES = new Map([
  ['progressbar', { rule: 'no-raw-progress', message: 'Raw role="progressbar" — use shadcn <Progress>.' }],
  ['dialog', { rule: 'no-raw-dialog', message: 'Raw role="dialog" — use shadcn <Dialog>.' }],
]);

const CONSOLE_RE = /\bconsole\.(log|debug|warn|info)\b/;
const TODO_RE = /\b(TODO|FIXME|HACK|XXX)\b/;
const MOCK_RE = /\b(vi\.fn|jest\.fn|vi\.mock|jest\.mock|vi\.spyOn|jest\.spyOn|sinon\.(stub|mock|fake))\b/;

const mode = process.argv.includes('--all') ? 'all' : 'changed';
const violations = [...findForbiddenAuditOkComments()];
const vueFiles = mode === 'all' ? collectAllVueFiles() : collectChangedFiles('.vue');
const tsFiles = mode === 'all' ? collectAllTsFiles() : collectChangedFiles('.ts');

for (const cssFile of collectSiblingCssFiles()) {
  addViolation(cssFile, 1, 'no-sibling-css', 'Sibling .css file — migrate styles to Tailwind utilities or tailwind.css global layer.');
}

for (const relPath of vueFiles) auditVueFile(relPath);
for (const relPath of tsFiles) auditCodeFile(relPath, false);

if (violations.length > 0) {
  console.error(`\n❌ Component audit: ${violations.length} violation(s) found:\n`);
  for (const [rule, items] of groupByRule(violations)) {
    console.error(`  ${rule} (${items.length}):`);
    for (const v of items) console.error(`    ${v.file}:${v.line}  ${v.message}`);
    console.error('');
  }
  console.error('Run with --all to scan every component.\n');
  process.exit(1);
}

const scanned = mode === 'all'
  ? `${vueFiles.length} components, ${tsFiles.length} TS files`
  : `${vueFiles.length} changed components, ${tsFiles.length} changed TS files`;
console.log(`✅ audit:components passed (${scanned}).`);

function auditVueFile(relPath) {
  const source = readFileSafe(path.join(APP_ROOT, relPath));
  if (source == null) return;

  const lines = source.split(/\r?\n/);
  const lineCount = countLinesForMaxLoc(source);
  if (lineCount > MAX_LOC) {
    addViolation(relPath, lineCount, 'max-loc', `${lineCount} lines exceeds ${MAX_LOC} limit — split this component.`);
  }

  for (const range of findBlockRanges(lines, 'style')) {
    if (STYLE_EXCEPTIONS.has(relPath)) continue;
    addViolation(relPath, range.start, 'no-style-block', '<style> blocks are banned by DESIGN.md — use Tailwind utilities or tailwind.css.');
  }

  for (const tag of findTemplateTags(lines)) auditTemplateTag(relPath, tag);
  auditCodeFile(relPath, true, lines);
}

function auditTemplateTag(relPath, tag) {
  const tagName = tag.name;
  const lowerTagName = tagName.toLowerCase();
  const isNativeTag = tagName === lowerTagName;
  const rawRule = isNativeTag && !isSanctionedFileInput(tag) ? RAW_CONTROL_BY_TAG.get(lowerTagName) : null;
  if (rawRule) addViolation(relPath, tag.line, rawRule.rule, rawRule.message);

  const role = findStaticAttr(tag.text, 'role')?.toLowerCase();
  const roleRule = role ? ROLE_RULES.get(role) : null;
  if (roleRule) addViolation(relPath, tag.line, roleRule.rule, roleRule.message);

  if (lowerTagName === 'component') {
    for (const hit of findDynamicRawControls(tag.text)) {
      addViolation(relPath, tag.line, hit.rule, hit.message);
    }
  }

  if (tagName === 'button' && /\bclass\s*=\s*"[^"]*\bbtn[-\s]/.test(tag.text)) {
    addViolation(relPath, tag.line, 'no-btn-classname', 'btn-* classname bypasses Button variant props — use variant/size props instead.');
  }
}

function isSanctionedFileInput(tag) {
  if (tag.name !== 'input') return false;
  const type = findStaticAttr(tag.text, 'type')?.toLowerCase();
  if (type !== 'file') return false;
  const className = findStaticAttr(tag.text, 'class') ?? '';
  return className.split(/\s+/).includes('sr-only');
}

function findDynamicRawControls(tagText) {
  const isAttr = tagText.match(/\b(:is|v-bind:is|is)\s*=\s*(["'])([\s\S]*?)\2/i);
  if (!isAttr) return [];

  const [, attrName, , value] = isAttr;
  const staticValue = attrName === 'is' ? value.trim().toLowerCase() : null;
  return RAW_CONTROL_RULES.flatMap(([rawTag, rawRule, rawMessage]) => {
    const rendersRaw = staticValue === rawTag || (staticValue == null && containsStringLiteral(value, rawTag));
    if (!rendersRaw) return [];
    return [{
      rule: rawRule.replace('no-raw-', 'no-dynamic-raw-'),
      message: rawMessage.replace('Raw <', `Dynamic <component :is> renders raw <`),
    }];
  });
}

function auditCodeFile(relPath, isVue, existingLines = null) {
  const source = existingLines?.join('\n') ?? readFileSafe(path.join(APP_ROOT, relPath));
  if (source == null) return;

  const maxLocViolation = checkMaxLoc(source, relPath, isVue);
  if (maxLocViolation) addViolation(
    maxLocViolation.file,
    maxLocViolation.line,
    maxLocViolation.rule,
    maxLocViolation.message
  );

  const lines = existingLines ?? source.split(/\r?\n/);

  const templateRanges = isVue ? findBlockRanges(lines, 'template') : [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i] ?? '';
    if (isVue && isInRanges(i + 1, templateRanges)) continue;
    if (/^\s*(\/\/|\*)/.test(line)) continue;
    if (CONSOLE_RE.test(line)) addViolation(relPath, i + 1, 'no-console', 'console.log/debug/warn — remove or use a proper logger.');
    if (TODO_RE.test(line)) addViolation(relPath, i + 1, 'no-todo', 'TODO/FIXME/HACK — resolve or file a gate task.');
    if (MOCK_RE.test(line)) addViolation(relPath, i + 1, 'no-mock', 'Mock/stub/fake — constitution forbids mocks.');
  }
}

export function checkMaxLoc(source, relPath, isVue) {
  if (isVue) return null;
  const lineCount = countLinesForMaxLoc(source);
  if (lineCount <= MAX_LOC) return null;
  return {
    file: relPath,
    line: lineCount,
    rule: 'max-loc',
    message: `${lineCount} lines exceeds ${MAX_LOC} limit — split this file.`,
  };
}

function addViolation(file, line, rule, message) {
  if (RAW_CONTROL_EXCEPTIONS.get(file)?.has(rule)) return;
  violations.push({ file, line, rule, message });
}

function findTemplateTags(lines) {
  const tags = [];
  for (const range of findBlockRanges(lines, 'template')) {
    const startLine = range.start + 1;
    const source = lines.slice(range.start, range.end - 1).join('\n')
      .replace(/<!--[\s\S]*?-->/g, (comment) => comment.replace(/[^\n]/g, ' '));
    const tagRe = /<(?![!/])([A-Za-z][\w.-]*)\b[\s\S]*?>/g;
    for (const match of source.matchAll(tagRe)) {
      tags.push({ name: match[1], text: match[0], line: startLine + countNewlines(source.slice(0, match.index)) });
    }
  }
  return tags;
}

function findBlockRanges(lines, blockName) {
  const openRe = new RegExp(`^\\s*<${blockName}\\b`, 'i');
  const closeRe = new RegExp(`^\\s*</${blockName}>`, 'i');
  if (blockName === 'template') {
    let start = null;
    let end = null;
    for (let i = 0; i < lines.length; i++) {
      if (start == null && openRe.test(lines[i] ?? '')) start = i + 1;
      if (start != null && closeRe.test(lines[i] ?? '')) end = i + 1;
    }
    return start == null ? [] : [{ start, end: end ?? lines.length }];
  }

  const ranges = [];
  let start = null;
  for (let i = 0; i < lines.length; i++) {
    if (start == null && openRe.test(lines[i] ?? '')) start = i + 1;
    else if (start != null && closeRe.test(lines[i] ?? '')) {
      ranges.push({ start, end: i + 1 });
      start = null;
    }
  }
  if (start != null) ranges.push({ start, end: lines.length });
  return ranges;
}

function findStaticAttr(tagText, attrName) {
  return tagText.match(new RegExp(`\\b${attrName}\\s*=\\s*(["'])(.*?)\\1`, 'i'))?.[2] ?? null;
}

function containsStringLiteral(expression, value) {
  const lower = expression.toLowerCase();
  return [`'${value}'`, `"${value}"`, `\`${value}\``].some((literal) => lower.includes(literal));
}

function collectAllVueFiles() {
  return collectFiles(COMPONENTS_DIR, '.vue');
}

function collectAllTsFiles() {
  return collectFiles(path.join(APP_ROOT, 'src'), '.ts');
}

function collectFiles(root, ext) {
  const results = [];
  walkDir(root, results, ext);
  return results.map((f) => path.relative(APP_ROOT, f).split(path.sep).join('/'));
}

function walkDir(dir, results, ext = null) {
  if (dir === UI_DIR || dir.startsWith(`${UI_DIR}${path.sep}`) || SKIP_DIRS.has(path.basename(dir))) return;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walkDir(full, results, ext);
    else if (entry.isFile() && (ext == null || entry.name.endsWith(ext))) results.push(full);
  }
}

function collectChangedFiles(ext) {
  const result = spawnSync('git', ['diff', '--name-only', '--relative', '--diff-filter=ACMR', 'HEAD~1...HEAD', '--', 'src/'], {
    cwd: APP_ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  if (result.status !== 0 || !result.stdout.trim()) return [];
  return result.stdout.trim().split('\n')
    .map((f) => f.replace(/^apps\/desktop\//, ''))
    .filter((f) => f.endsWith(ext) && !f.includes('/ui/') && !f.includes('__tests__'));
}

function collectSiblingCssFiles() {
  try {
    return readdirSync(COMPONENTS_DIR, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith('.css'))
      .map((entry) => `src/components/${entry.name}`);
  } catch {
    return [];
  }
}

function findForbiddenAuditOkComments() {
  const files = [];
  const found = [];
  walkDir(path.join(APP_ROOT, 'src'), files);
  for (const filePath of files) {
    const source = readFileSafe(filePath);
    if (source == null || !source.includes('audit-ok')) continue;
    const relPath = path.relative(APP_ROOT, filePath).split(path.sep).join('/');
    source.split(/\r?\n/).forEach((line, index) => {
      if (line.includes('audit-ok')) {
        found.push({ file: relPath, line: index + 1, rule: 'no-audit-ok', message: 'audit-ok comments are banned. Remove the comment and fix the underlying violation.' });
      }
    });
  }
  return found;
}

function isInRanges(lineNumber, ranges) {
  return ranges.some((r) => lineNumber >= r.start && lineNumber <= r.end);
}

function countNewlines(text) {
  return (text.match(/\n/g) ?? []).length;
}

function groupByRule(items) {
  const grouped = new Map();
  for (const item of items) {
    if (!grouped.has(item.rule)) grouped.set(item.rule, []);
    grouped.get(item.rule).push(item);
  }
  return grouped;
}

function readFileSafe(filePath) {
  try {
    return readFileSync(filePath, 'utf8');
  } catch {
    return null;
  }
}
