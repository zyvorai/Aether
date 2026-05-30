// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { AppView } from '../types/api';

export interface AiOsNavSection {
  id: string;
  label: string;
  primaryView: AppView;
  /** Views that belong to this section (for active-state highlighting). */
  views: AppView[];
}

/** Twelve-section AI OS navigation — see docs/VISION-AI-OS.md */
export const AI_OS_NAV: AiOsNavSection[] = [
  {
    id: 'overview',
    label: 'Overview',
    primaryView: 'overview',
    views: ['overview'],
  },
  {
    id: 'fleet',
    label: 'Fleet',
    primaryView: 'fleet',
    views: ['fleet', 'activity', 'intelligence', 'hosted'],
  },
  {
    id: 'fabric',
    label: 'Fabric',
    primaryView: 'fabric',
    views: ['fabric'],
  },
  {
    id: 'workloads',
    label: 'Workloads',
    primaryView: 'workloads',
    views: ['workloads', 'applications', 'editor', 'templates', 'compose', 'scheduler'],
  },
  {
    id: 'ai-studio',
    label: 'AI Studio',
    primaryView: 'ai',
    views: ['ai', 'copilot', 'affinity', 'confidential'],
  },
  {
    id: 'migrations',
    label: 'Migrations',
    primaryView: 'migrations',
    views: ['migrations'],
  },
  {
    id: 'observability',
    label: 'Observability',
    primaryView: 'observability',
    views: ['observability', 'health', 'events', 'metrics', 'alerts', 'sla'],
  },
  {
    id: 'security',
    label: 'Security',
    primaryView: 'security',
    views: ['security', 'rbac', 'audit', 'policy'],
  },
  {
    id: 'cost',
    label: 'Cost',
    primaryView: 'cost',
    views: ['cost'],
  },
  {
    id: 'gitops',
    label: 'GitOps',
    primaryView: 'gitops',
    views: ['gitops', 'drift'],
  },
  {
    id: 'labs',
    label: 'Labs',
    primaryView: 'labs',
    views: ['labs', 'helm', 'openapi', 'deps'],
  },
  {
    id: 'settings',
    label: 'Settings',
    primaryView: 'settings',
    views: ['settings', 'platform', 'envs', 'secrets', 'backups', 'plugins', 'clusters'],
  },
];

export function aiOsSectionForView(view: AppView): AiOsNavSection | undefined {
  return AI_OS_NAV.find((s) => s.views.includes(view));
}

export function isAiOsNavActive(section: AiOsNavSection, currentView: AppView): boolean {
  return section.views.includes(currentView);
}
