// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { moveIndex, nextSort, sortByValue } from './tableSort';

describe('table sorting', () => {
  it('cycles ascending, descending, unsorted', () => {
    const asc = nextSort(null, 'name');
    expect(asc).toEqual({ key: 'name', dir: 'asc' });
    const desc = nextSort(asc, 'name');
    expect(desc).toEqual({ key: 'name', dir: 'desc' });
    expect(nextSort(desc, 'name')).toBeNull();
    expect(nextSort(desc, 'status')).toEqual({ key: 'status', dir: 'asc' });
  });

  it('sorts text naturally and leaves the input alone', () => {
    const items = ['node-10', 'Node-9', 'node-1'];
    expect(sortByValue(items, (v) => v, 'asc')).toEqual(['node-1', 'Node-9', 'node-10']);
    expect(sortByValue(items, (v) => v, 'desc')).toEqual(['node-10', 'Node-9', 'node-1']);
    expect(items).toEqual(['node-10', 'Node-9', 'node-1']);
  });

  it('sorts numbers by value', () => {
    expect(sortByValue([10, 2, -1], (v) => v, 'asc')).toEqual([-1, 2, 10]);
  });

  it('moves the active row within bounds', () => {
    expect(moveIndex('j', 0, 3)).toBe(1);
    expect(moveIndex('ArrowDown', 2, 3)).toBe(2);
    expect(moveIndex('k', 0, 3)).toBe(0);
    expect(moveIndex('End', 0, 3)).toBe(2);
    expect(moveIndex('Home', 2, 3)).toBe(0);
    expect(moveIndex('x', 1, 3)).toBeNull();
    expect(moveIndex('j', 0, 0)).toBeNull();
  });
});
