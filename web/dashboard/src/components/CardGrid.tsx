// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

type ColumnPreset = 'compact' | 'default' | 'wide';

const columnClass: Record<ColumnPreset, string> = {
  // Denser cards (secrets, small entities)
  compact: 'grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4',
  // Standard inventory cards
  default: 'grid-cols-1 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4',
  // Larger info panels
  wide: 'grid-cols-1 lg:grid-cols-2 2xl:grid-cols-3',
};

interface CardGridProps {
  children: ReactNode;
  columns?: ColumnPreset;
  className?: string;
  testId?: string;
}

export default function CardGrid({ children, columns = 'default', className, testId }: CardGridProps) {
  return (
    <div className={`grid gap-3 ${columnClass[columns]} ${className ?? ''}`} data-testid={testId}>
      {children}
    </div>
  );
}

/** Stagger delay for entrance animation; pair with the `card-rise` class. */
export function cardRiseDelay(index: number): string {
  return `${Math.min(index, 12) * 35}ms`;
}
