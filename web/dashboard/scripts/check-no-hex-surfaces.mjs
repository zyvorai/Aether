#!/usr/bin/env node
// Fail if inline hex background surfaces remain in dashboard TSX sources.
import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

const root = join(import.meta.dirname, '..', 'src');

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

const pattern = /bg-\[#/g;
const files = await walk(root);
const violations = [];

for (const file of files) {
  const text = await readFile(file, 'utf8');
  if (pattern.test(text)) violations.push(file.replace(root + '/', 'src/'));
}

if (violations.length) {
  console.error('Hex surface backgrounds found (use glass-inset-surface or glass-input):');
  for (const v of violations) console.error(`  ${v}`);
  process.exit(1);
}

console.log('OK: no bg-[# hex surfaces in dashboard TSX');
