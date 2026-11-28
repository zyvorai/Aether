// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { AppView } from '../types/api';
import { getViewMeta } from './dashboardNav';
import { getParentView } from './navSections';

/** "Sbom · Security Center · Aether": the page first, so it survives a narrow tab. */
export function pageTitle(view: AppView): string {
  const meta = getViewMeta(view);
  const parent = getParentView(view);
  return [meta.label, parent ? getViewMeta(parent).label : null, 'Aether'].filter(Boolean).join(' · ');
}
