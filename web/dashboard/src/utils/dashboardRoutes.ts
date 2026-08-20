// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { AppView } from '../types/api';
import { VIEW_TO_PATH as NAV_VIEW_TO_PATH } from './dashboardNav';

export const VIEW_TO_PATH = NAV_VIEW_TO_PATH;

const PATH_TO_VIEW = new Map<string, AppView>();
for (const [view, path] of Object.entries(VIEW_TO_PATH) as [AppView, string][]) {
  PATH_TO_VIEW.set(path, view);
}

export function viewToPath(view: AppView): string {
  return VIEW_TO_PATH[view];
}

export function pathToView(pathname: string): AppView | null {
  const normalized = pathname.replace(/\/+$/, '') || '/';
  if (normalized === '/copilot') return 'zyra';
  return PATH_TO_VIEW.get(normalized) ?? null;
}
