// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { ComponentType, ReactNode } from 'react';
import type { LucideIcon } from 'lucide-react';
import { PageHero, type Tone } from '../layout/PageHero';
import { getViewMeta } from '../../utils/dashboardNav';
import type { AppView } from '../../types/api';
import { cn } from '../../lib/cn';
import { sectionTone } from '../../utils/sectionTone';
import SectionTabs from '../SectionTabs';

export type AuroraPageProps = {
  view: AppView;
  /** Override title (defaults to dashboardNav label). */
  title?: string;
  /** Override subtitle. */
  description?: string;
  actions?: ReactNode;
  icon?: LucideIcon;
  accent?: Tone;
  swatches?: { label: string; tone: Tone; active?: boolean; onSelect?: () => void }[];
  stats?: { label: string; value: string | number; tone?: Tone }[];
  /** Optional test id for the ink highlights band. */
  statsTestId?: string;
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
  swatches,
  stats,
  statsTestId,
  className,
  children,
}: AuroraPageProps) {
  const meta = getViewMeta(view);
  return (
    <div className={cn('page-stack animate-fade-up', className)} data-testid={`page-${view}`}>
      <PageHero
        title={title ?? meta.label}
        description={description ?? meta.subtitle}
        actions={actions}
        icon={icon ?? meta.icon}
        tileTone={accent ?? sectionTone(view)}
        accent={accent}
        swatches={swatches}
        stats={stats}
        statsTestId={statsTestId}
      />
      <SectionTabs currentView={view} />
      <div className="page-stack">{children}</div>
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
