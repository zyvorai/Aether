// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { entersTable } from './tableSort';

const key = (k: string, extra: Partial<KeyboardEvent> = {}) => ({ key: k, metaKey: false, ctrlKey: false, altKey: false, defaultPrevented: false, ...extra });

describe('reaching a table with j and k', () => {
  it('works before any row has focus', () => {
    expect(entersTable(key('j'), { tagName: 'BODY' }, false)).toBe(true);
    expect(entersTable(key('k'), { tagName: 'A' }, false)).toBe(true);
    expect(entersTable(key('j'), null, false)).toBe(true);
  });

  it('stays out of the way of typing, dialogs, shortcuts and other handlers', () => {
    for (const tagName of ['INPUT', 'TEXTAREA', 'SELECT']) expect(entersTable(key('j'), { tagName }, false)).toBe(false);
    expect(entersTable(key('j'), { tagName: 'DIV', isContentEditable: true }, false)).toBe(false);
    expect(entersTable(key('j'), { tagName: 'BODY' }, true)).toBe(false);
    expect(entersTable(key('k', { metaKey: true }), { tagName: 'BODY' }, false)).toBe(false);
    expect(entersTable(key('j', { defaultPrevented: true }), { tagName: 'DIV' }, false)).toBe(false);
    expect(entersTable(key('ArrowDown'), { tagName: 'BODY' }, false)).toBe(false);
  });
});
