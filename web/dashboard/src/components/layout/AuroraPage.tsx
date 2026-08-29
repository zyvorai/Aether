// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ComponentType, ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';
import { PageHero, type Tone } from '../layout/PageHero';
import { getViewMeta, type NavGroup } from '../../utils/dashboardNav';
import type { AppView } from '../../types/api';
import { cn } from '../../lib/cn';

const GROUP_EYEBROW: Record<NavGroup, string> = {
  primary: 'Workspace',
  intelligence: 'Intelligence',
  operations: 'Operations',
  resources: 'Resources',
};

export type AuroraPageProps = {
  view: AppView;
  /** Override title (defaults to dashboardNav label). */
  title?: string;
  /** Override subtitle. */
  description?: string;
  actions?: ReactNode;
  icon?: LucideIcon;
  accent?: Tone;
  stats?: { label: string; value: string | number; tone?: Tone }[];
  className?: string;
  children: ReactNode;
};

/** Aurora page composition: PageHero + content stack. Use on every dashboard page. */
export default function AuroraPage({
  view,
  title,
  description,
  actions,
  icon,
  accent,
  stats,
  className,
  children,
}: AuroraPageProps) {
  const meta = getViewMeta(view);
  return (
    <div className={cn('space-y-8 animate-fade-up', className)} data-testid={`page-${view}`}>
      <PageHero
        eyebrow={GROUP_EYEBROW[meta.group]}
        title={title ?? meta.label}
        description={description ?? meta.subtitle}
        actions={actions}
        icon={icon}
        accent={accent}
        stats={stats}
      />
      <div className="space-y-6">{children}</div>
    </div>
  );
}

/** Wrap a page component so every render state shares the Aurora composition. */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function withAuroraPage(view: AppView, Page: ComponentType<any>): ComponentType<any> {
  function AuroraPageBoundary(props: Record<string, unknown>) {
    return (
      <AuroraPage view={view}>
        <Page {...props} />
      </AuroraPage>
    );
  }

  AuroraPageBoundary.displayName = `AuroraPage(${Page.displayName ?? Page.name ?? view})`;
  return AuroraPageBoundary;
}
