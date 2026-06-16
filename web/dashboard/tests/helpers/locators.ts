// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

import type { Page } from '@playwright/test';

/** Prefer first match when duplicate test ids exist in hub + scoped banners. */
export function testId(page: Page, id: string) {
  return page.getByTestId(id).first();
}
