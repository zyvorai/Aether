import type { LucideIcon } from 'lucide-react';
import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Eyebrow, PageTitle, TextLead } from '../ui/Typography';

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
    <section className={cn('tahoe-hero space-y-6', className)}>
      <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-2 min-w-0 max-w-2xl">
          {eyebrow ? <Eyebrow>{eyebrow}</Eyebrow> : null}
          <PageTitle>{title}</PageTitle>
          {description ? <TextLead>{description}</TextLead> : null}
        </div>
        {actions ? <div className="flex flex-wrap gap-2 shrink-0">{actions}</div> : null}
      </div>
      {stats?.length ? (
        <div className="grid grid-cols-2 lg:grid-cols-4 gap-6 pt-2 border-t border-border">
          {stats.map((stat) => <div key={stat.label} className="tahoe-stat-tile"><p className="tahoe-stat-value">{stat.value}</p><p className="tahoe-stat-label mt-1">{stat.label}</p></div>)}
        </div>
      ) : null}
    </section>
  );
}
