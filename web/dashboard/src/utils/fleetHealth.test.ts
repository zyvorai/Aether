// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { describe, expect, it } from 'vitest';
import { fleetBreakdown, fleetPercent, fleetSubtitle } from './fleetHealth';

const fleet = (running: number, unhealthy: number, stopped: number, unknown: number) => ({
  total: running + unhealthy + stopped + unknown,
  running,
  unhealthy,
  stopped,
  unknown,
});

describe('fleet health wording', () => {
  it('shows no percentage without a current check', () => {
    expect(fleetPercent(null)).toBe('Not checked');
    expect(fleetPercent(0)).toBe('0%');
    expect(fleetSubtitle(0, null, fleet(0, 0, 0, 3))).toBe('3 of 3 workloads have no current health check');
  });

  it('does not call a partly checked fleet healthy', () => {
    expect(fleetSubtitle(0, 100, fleet(1, 0, 0, 1))).toBe('1 of 2 workloads has no current health check');
    expect(fleetSubtitle(1, 80, fleet(4, 1, 0, 0))).toBe('1 signal needs attention · Fleet at 80% health');
  });

  it('counts each group', () => {
    expect(fleetBreakdown(fleet(3, 1, 1, 0))).toBe('3 running · 1 in trouble · 1 stopped');
    expect(fleetBreakdown(fleet(0, 0, 0, 0))).toBe('Deploy workloads to begin monitoring');
  });

  it('says when everything is stopped on purpose', () => {
    expect(fleetSubtitle(0, null, fleet(0, 0, 2, 0))).toBe('Every workload is stopped on request');
  });
});
