// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { clockTime, groupByDay } from './eventGroups';

describe('groupByDay', () => {
  const now = new Date(2026, 2, 10, 12, 0, 0);
  const at = (day: number, hour: number) => ({ timestamp: new Date(2026, 2, day, hour).toISOString(), id: `${day}-${hour}` });

  it('groups by local day, newest first', () => {
    const groups = groupByDay([at(9, 8), at(10, 9), at(2, 1), at(10, 11)], now);
    expect(groups.map((g) => g.label)).toEqual(['Today', 'Yesterday', '2026-03-02']);
    expect(groups[0].items.map((i) => i.id)).toEqual(['10-11', '10-9']);
  });

  it('keeps unreadable timestamps visible', () => {
    expect(groupByDay([{ timestamp: 'soon' }], now)[0].label).toBe('Unknown date');
    expect(clockTime('soon')).toBe('soon');
  });
});
