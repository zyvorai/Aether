#!/usr/bin/env node
/**
 * Cross-check example workloads against schema/workload.schema.json using ajv.
 * Rust `aether validate` remains the source of truth; this catches IDE-schema drift.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dashboardRoot = join(root, 'web', 'dashboard');
const require = createRequire(join(dashboardRoot, 'package.json'));
const Ajv = require('ajv');
const { parse: parseYaml } = require('yaml');
const schemaPath = join(root, 'schema', 'workload.schema.json');
const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));
const ajv = new Ajv({ allErrors: true, strict: false });
const validate = ajv.compile(schema);

const examples = [
  'examples/workload-full-featured.yaml',
  'examples/workload-k8s-advanced.yaml',
  'examples/labs/kubernetes/workload.yaml',
];

let failed = false;
for (const rel of examples) {
  const path = join(root, rel);
  const doc = parseYaml(readFileSync(path, 'utf8'));
  const ok = validate(doc);
  if (!ok) {
    failed = true;
    console.error(`FAIL ${rel}:`);
    for (const err of validate.errors ?? []) {
      console.error(`  ${err.instancePath || '/'} ${err.message}`);
    }
  } else {
    console.log(`  ok: ${rel}`);
  }
}

if (failed) process.exit(1);
