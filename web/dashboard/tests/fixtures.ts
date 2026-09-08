// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { test as base, expect } from '@playwright/test';
export type { Page, Locator, APIRequestContext } from '@playwright/test';

/** Prefer first match when duplicate data-testid values exist (hub + scoped banners). */
export const test = base.extend({
  page: async ({ page }, use) => {
    await page.addInitScript(() => {
      const hideDock = () => {
        document.querySelectorAll('[data-testid="agent-status-dock"]').forEach((el) => {
          (el as HTMLElement).style.display = 'none';
        });
      };
      hideDock();
      new MutationObserver(hideDock).observe(document.documentElement, { childList: true, subtree: true });
    });
    const getByTestId = page.getByTestId.bind(page);
    page.getByTestId = (testId: string, options?: Parameters<typeof getByTestId>[1]) => {
      const locator = getByTestId(testId, options).first();
      const baseClick = locator.click.bind(locator);
      locator.click = async (clickOptions?: Parameters<typeof baseClick>[0]) => {
        await locator.scrollIntoViewIfNeeded();
        return baseClick(clickOptions);
      };
      return locator;
    };
    const goto = page.goto.bind(page);
    page.goto = async (url, options) => {
      const res = await goto(url, options);
      const path = typeof url === 'string' ? url.split('?')[0] : '';
      if (path === '/' || path === '') {
        await page
          .locator('[data-testid="overview-platform-quick-link"]')
          .scrollIntoViewIfNeeded()
          .catch(() => undefined);
      }
      return res;
    };
    await use(page);
  },
});

export { expect };
