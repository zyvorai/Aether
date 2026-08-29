import type { LucideIcon } from 'lucide-react';
import { Button } from './Button';
import { SectionTitle, TextMuted } from './Typography';
import { cn } from '../../lib/cn';
import type { Tone } from '../layout/PageHero';

interface EmptyStateAction {
  label: string;
  onClick: () => void;
  primary?: boolean;
}

const toneText: Record<Tone, string> = {
  sky: 'text-primary', violet: 'text-accent-purple', emerald: 'text-success',
  amber: 'text-warning', pink: 'text-accent-pink', teal: 'text-accent-teal', rust: 'text-primary',
};

export function EmptyState({
  icon: Icon, title, description, actions, tone, className,
}: {
  icon?: LucideIcon; title: string; description?: string; actions?: EmptyStateAction[];
  tone?: Tone; className?: string;
}) {
  return (
    <div className={cn('tahoe-empty animate-fade-up py-16 px-6 text-center', className)}>
      <div className="tahoe-empty-orb" aria-hidden />
      <div className="relative">
        {Icon ? <div className={cn('tahoe-empty-icon mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full', tone ? toneText[tone] : 'text-primary')}><Icon className="h-6 w-6" /></div> : null}
        <SectionTitle className="mb-2">{title}</SectionTitle>
        {description ? <TextMuted className="mx-auto mb-6 max-w-md">{description}</TextMuted> : null}
        {actions?.length ? <div className="flex flex-wrap justify-center gap-3">{actions.map((action) => <Button key={action.label} size="lg" variant={action.primary === false ? 'secondary' : 'primary'} onClick={action.onClick}>{action.label}</Button>)}</div> : null}
      </div>
    </div>
  );
}
