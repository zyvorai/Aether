// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { AppView, PlatformInfo } from '../types/api';

export interface NavVisibility {
  visible: boolean;
  dimmed?: boolean;
  setupHint?: string;
}

export interface NavCapabilityExtras {
  gitopsConfigured?: boolean | null;
}

function normalizeStrategyHint(view: AppView, platform: PlatformInfo | null | undefined): NavVisibility {
  if (!platform) return { visible: true };

  switch (view) {
    case 'policy':
      if (platform.opa?.configured) return { visible: true };
      return {
        visible: false,
        setupHint: 'Configure OPA on Platform & HA before using Policy Check',
      };
    case 'gitops':
      if (platform.integrations?.grafana_url) {
        /* gitops independent of grafana — use extras */
      }
      return { visible: true };
    case 'metrics':
      if (platform.integrations?.prometheus_url) return { visible: true };
      return {
        visible: true,
        dimmed: true,
        setupHint: 'Set Prometheus URL on Platform & HA for live metrics links',
      };
    default:
      return { visible: true };
  }
}

export function navVisibilityForView(
  view: AppView,
  platform: PlatformInfo | null | undefined,
  extras?: NavCapabilityExtras,
): NavVisibility {
  const base = normalizeStrategyHint(view, platform);

  if (view === 'gitops' && extras?.gitopsConfigured === false) {
    return {
      visible: false,
      setupHint: 'Run aether git-ops init on the server to enable GitOps',
    };
  }

  return base;
}

export function filterNavViews<T extends { view: AppView }>(
  items: T[],
  platform: PlatformInfo | null | undefined,
  extras?: NavCapabilityExtras,
): T[] {
  return items.filter((item) => navVisibilityForView(item.view, platform, extras).visible);
}

export type AnnotatedNavItem<T extends { view: AppView }> = T & { visibility: NavVisibility };

/** Split nav items into ready-to-use vs setup-required (shown dimmed with platform CTA). */
export function partitionNavViews<T extends { view: AppView }>(
  items: T[],
  platform: PlatformInfo | null | undefined,
  extras?: NavCapabilityExtras,
): { ready: AnnotatedNavItem<T>[]; setup: AnnotatedNavItem<T>[] } {
  const ready: AnnotatedNavItem<T>[] = [];
  const setup: AnnotatedNavItem<T>[] = [];
  for (const item of items) {
    const visibility = navVisibilityForView(item.view, platform, extras);
    const annotated = { ...item, visibility };
    if (visibility.visible && !visibility.dimmed) {
      ready.push(annotated);
    } else {
      setup.push(annotated);
    }
  }
  return { ready, setup };
}

export function platformSetupNeeded(platform: PlatformInfo | null | undefined): string[] {
  if (!platform) return [];
  const hints: string[] = [];
  if (!platform.opa?.configured) {
    hints.push('Policy Check requires OPA — configure on Platform & HA');
  }
  if (!platform.integrations?.prometheus_url) {
    hints.push('Metrics integrations — add Prometheus URL on Platform & HA');
  }
  return hints;
}
