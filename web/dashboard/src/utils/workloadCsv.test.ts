// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { csvEscape, workloadsToCsv } from './workloadCsv';

describe('workloadCsv', () => {
  it('escapes quotes and commas', () => {
    expect(csvEscape('plain')).toBe('plain');
    expect(csvEscape('a,b')).toBe('"a,b"');
    expect(csvEscape('say "hi"')).toBe('"say ""hi"""');
  });

  it('builds a headered CSV for workload rows', () => {
    const csv = workloadsToCsv([
      {
        name: 'demo',
        kind: 'Deployment',
        runtime: 'kubernetes',
        image: 'nginx:1.25',
        status: 'Running',
        cluster: 'prod',
        namespace: 'web',
        source: 'cluster',
        created_at: '2026-07-26T10:00:00Z',
      },
    ]);
    expect(csv.startsWith('name,kind,runtime,image,status,cluster,namespace,source,created_at\n')).toBe(true);
    expect(csv).toContain('demo,Deployment,kubernetes,nginx:1.25,Running,prod,web,cluster,');
  });
});
