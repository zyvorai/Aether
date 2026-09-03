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

interface PageHeroProps {
  eyebrow?: string;
  title: string;
  description?: string;
  actions?: ReactNode;
  icon?: LucideIcon;
  accent?: Tone;
  stats?: HeroStat[];
  className?: string;
}

export function PageHero({ eyebrow, title, description, actions, stats, className }: PageHeroProps) {
  return (
    <section className={cn('tahoe-hero apple-editorial-hero space-y-6', className)}>
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-2 min-w-0 max-w-3xl">
          {eyebrow ? <Eyebrow>{eyebrow}</Eyebrow> : null}
          <DisplayTitle>{title}</DisplayTitle>
          {description ? <TextLead>{description}</TextLead> : null}
        </div>
        {actions ? <div className="flex flex-wrap gap-2 shrink-0">{actions}</div> : null}
      </div>
      {stats?.length ? (
        <div className="apple-chapter-dark marketplace-chapter-ink rounded-2xl px-6 py-8">
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
