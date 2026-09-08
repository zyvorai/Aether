// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { describe, expect, it } from 'vitest';
import { isApplyableRecommendation } from './troubleshootActions';

describe('troubleshootActions', () => {
  it('marks backend applyable recommendations as applyable', () => {
    expect(
      isApplyableRecommendation({
        title: 'Restart',
        summary: 'x',
        action: 'restart',
        applyable: true,
      }),
    ).toBe(true);
  });

  it('treats navigation-only actions as not applyable', () => {
    expect(
      isApplyableRecommendation({
        title: 'Network',
        summary: 'x',
        action: 'check_network',
        applyable: false,
      }),
    ).toBe(false);
  });
});
