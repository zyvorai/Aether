import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';
import { Eyebrow, SectionTitle, TextSmall } from '../ui/Typography';

interface SectionHeaderProps {
  label?: string;
  title: ReactNode;
  description?: string;
  action?: ReactNode;
  className?: string;
}

export function SectionHeader({ label, title, description, action, className }: SectionHeaderProps) {
  return (
    <div className={cn('mb-4 flex flex-col gap-1 sm:flex-row sm:items-end sm:justify-between', className)}>
      <div>
        {label ? <Eyebrow className="mb-1">{label}</Eyebrow> : null}
        <SectionTitle>{title}</SectionTitle>
        {description ? <TextSmall className="mt-1">{description}</TextSmall> : null}
      </div>
      {action}
    </div>
  );
}
