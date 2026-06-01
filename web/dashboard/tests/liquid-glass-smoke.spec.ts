// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import { test, expect } from '@playwright/test';

const GLASS_ROUTES = [
  { path: '/', shell: '.command-center-shell, .overview-section-shell' },
  { path: '/clusters', shell: '.overview-section-shell' },
  { path: '/workloads', shell: '.overview-section-shell' },
  { path: '/editor', shell: '.overview-section-shell' },
  { path: '/helm', shell: '.overview-section-shell' },
  { path: '/secrets', shell: '.overview-section-shell' },
  { path: '/settings', shell: '.overview-section-shell' },
];

test.describe('Liquid Glass smoke', () => {
  for (const { path, shell } of GLASS_ROUTES) {
    test(`glass shell visible with blur on ${path}`, async ({ page }) => {
      await page.goto(path);

      const glassShell = page.locator(shell).first();
      await expect(glassShell).toBeVisible({ timeout: 15_000 });

      const backdropFilter = await glassShell.evaluate((el) => getComputedStyle(el).backdropFilter);
      expect(backdropFilter).not.toBe('none');
    });
  }
});
