import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

export type BadgeVariant = 'default' | 'success' | 'warning' | 'danger' | 'tier-a' | 'tier-b' | 'tier-c' | 'blue' | 'pink' | 'teal' | 'purple';

const variants: Record<BadgeVariant, string> = {
  default: 'bg-surface text-muted',
  success: 'bg-surface text-success',
  warning: 'bg-surface text-warning',
  danger: 'bg-surface text-danger',
  'tier-a': 'bg-surface text-success',
  'tier-b': 'bg-surface text-warning',
  'tier-c': 'bg-surface text-muted',
  blue: 'bg-surface text-primary',
  pink: 'bg-surface text-accent-pink',
  teal: 'bg-surface text-accent-teal',
  purple: 'bg-surface text-accent-purple',
};

export function Badge({ children, variant = 'default', className }: { children: ReactNode; variant?: BadgeVariant; className?: string }) {
  return <span className={cn('inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium', variants[variant], className)}>{children}</span>;
}

export function tierBadgeVariant(tier: string): BadgeVariant {
  return tier === 'A' ? 'tier-a' : tier === 'B' ? 'tier-b' : 'tier-c';
}
