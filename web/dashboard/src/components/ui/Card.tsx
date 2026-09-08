// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

interface CardProps {
  children: ReactNode;
  className?: string;
  elevated?: boolean;
  strong?: boolean;
  hover?: boolean;
  'data-testid'?: string;
}

export function Card({ children, className, elevated, strong, hover, 'data-testid': testId }: CardProps) {
  return (
    <div
      data-testid={testId}
      className={cn('glass', elevated && 'glass-elevated', strong && 'glass-strong', hover && 'glass-hover-lift', className)}
    >
      {children}
    </div>
  );
}

export function CardHeader({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn('px-5 py-4 border-b border-border', className)}>{children}</div>;
}

export function CardBody({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn('p-5', className)}>{children}</div>;
}

export function CardFooter({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn('px-5 py-4 border-t border-border', className)}>{children}</div>;
}
