// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { MARK_PATH, MARK_VIEWBOX, WORDMARK_PATHS } from './geometry';

const read = (path: string) => readFileSync(new URL(`../../../../../${path}`, import.meta.url), 'utf8');

describe('Aether brand geometry', () => {
  it('draws the favicon from the same mark as the component', () => {
    const favicon = read('web/dashboard/public/favicon.svg');
    expect(favicon).toContain(`d="${MARK_PATH}"`);
    expect(favicon).toContain(`viewBox="${MARK_VIEWBOX}"`);
    expect(favicon).toContain('prefers-color-scheme:dark');
  });

  it('keeps both README lockups in step with the mark and wordmark', () => {
    for (const theme of ['light', 'dark']) {
      const lockup = read(`docs/assets/readme/aether-lockup-${theme}.svg`);
      for (const d of [MARK_PATH, ...WORDMARK_PATHS]) expect(lockup).toContain(`d="${d}"`);
    }
  });
});
