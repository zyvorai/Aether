// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { DisplayTitle, Eyebrow, TextLead } from '../ui/Typography';
import AppleHighlightsRow from '../ui/AppleHighlightsRow';

export type Tone = 'sky' | 'violet' | 'emerald' | 'amber' | 'pink' | 'teal' | 'rust';

interface HeroStat {
  label: string;
  value: string | number;
  tone?: Tone;
}

/** iPad-style swatch: a colored pill under the hero description — a picker
 * when `onSelect`/`active` are wired, otherwise a plain legend chip. */
interface HeroSwatch {
  label: string;
  tone: Tone;
  active?: boolean;
  onSelect?: () => void;
}

interface PageHeroProps {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
  icon?: LucideIcon;
  accent?: Tone;
  swatches?: HeroSwatch[];
  stats?: HeroStat[];
  /** Optional test id for the ink highlights band (e.g. overview-apple-highlights). */
  statsTestId?: string;
  className?: string;
}

export function PageHero({ eyebrow, title, description, actions, accent, swatches, stats, statsTestId, className }: PageHeroProps) {
  return (
    <section data-tone={accent} className={cn('tahoe-hero apple-editorial-hero space-y-10', className)}>
      <div className="flex flex-col gap-6 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-3 min-w-0 max-w-3xl">
          {eyebrow ? <Eyebrow>{eyebrow}</Eyebrow> : null}
          <DisplayTitle>{title}</DisplayTitle>
          {description ? <TextLead>{description}</TextLead> : null}
          {swatches?.length ? (
            <div className="hero-swatch-row pt-1" role={swatches.some((s) => s.onSelect) ? 'group' : undefined}>
              {swatches.map((swatch) =>
                swatch.onSelect ? (
                  <button
                    key={swatch.label}
                    type="button"
                    data-tone={swatch.tone}
                    className="hero-swatch"
                    aria-pressed={swatch.active}
                    onClick={swatch.onSelect}
                  >
                    <span className="hero-swatch-dot" aria-hidden />
                    {swatch.label}
                  </button>
                ) : (
                  <span key={swatch.label} data-tone={swatch.tone} className="hero-swatch">
                    <span className="hero-swatch-dot" aria-hidden />
                    {swatch.label}
                  </span>
                ),
              )}
            </div>
          ) : null}
        </div>
        {actions ? <div className="flex flex-wrap gap-3 shrink-0">{actions}</div> : null}
      </div>
      {stats?.length ? (
        <div
          data-tone={accent}
          className="apple-chapter-dark marketplace-chapter-ink rounded-2xl px-6 py-12 sm:px-10 sm:py-14"
          data-testid={statsTestId}
        >
          <AppleHighlightsRow
            title="Get the highlights."
            items={stats.map((stat) => ({
              id: String(stat.label),
              value: String(stat.value),
              label: stat.label,
            }))}
          />
        </div>
      ) : null}
    </section>
  );
}
