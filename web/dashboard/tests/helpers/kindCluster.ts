// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { expect, type Page } from '@playwright/test';
import { ensureAuthenticated } from './auth';

export async function openNginxPlaywrightPodTerminal(page: Page) {
  await ensureAuthenticated(page);
  // The resource browser is its own page; /clusters only links to it.
  await page.goto('/clusters/browse');

  const clusterSelect = page.getByTestId('clusters-cluster-select');
  await clusterSelect.waitFor({ timeout: 30_000 });
  const options = await clusterSelect.locator('option').allTextContents();
  const kindIndex = options.findIndex((label) => /kind-/i.test(label));
  if (kindIndex >= 0) {
    await clusterSelect.selectOption({ index: kindIndex });
  }

  await page.getByTestId('clusters-namespace-select').selectOption('default');
  await page.getByTestId('clusters-kind-select').selectOption('Pod');
  await page.getByTestId('clusters-resource-table').waitFor({ timeout: 30_000 });

  await page.getByText('nginx-playwright').first().click({ timeout: 30_000 });
  const terminalTab = page.getByRole('tab', { name: 'Terminal' });
  await expect(terminalTab).toBeVisible({ timeout: 15_000 });
  await terminalTab.click();
}
