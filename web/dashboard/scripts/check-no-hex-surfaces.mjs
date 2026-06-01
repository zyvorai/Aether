#!/usr/bin/env node
// Fail if legacy inline surface backgrounds remain in dashboard TSX sources.
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..', 'src');

const rules = [
  { name: 'hex surfaces (bg-[#…])', pattern: /bg-\[#/g },
  { name: 'solid black fills (bg-black)', pattern: /\bbg-black(?:\/|\b)/g },
];

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) files.push(...(await walk(path)));
    else if (entry.name.endsWith('.tsx')) files.push(path);
  }
  return files;
}

const files = await walk(root);
const violations = [];

for (const file of files) {
  const text = await readFile(file, 'utf8');
  const rel = file.replace(root + '/', 'src/');
  for (const { name, pattern } of rules) {
    if (pattern.test(text)) violations.push({ file: rel, rule: name });
  }
}

if (violations.length) {
  console.error('Legacy surface backgrounds found:');
  for (const v of violations) console.error(`  ${v.file} — ${v.rule}`);
  console.error('Use glass-inset-surface, glass-input, or glass-code-block-body instead.');
  process.exit(1);
}

console.log('OK: no legacy hex/black surface fills in dashboard TSX');
