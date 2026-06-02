// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';

interface ResponsiveTableProps {
  children: ReactNode;
  className?: string;
  testId?: string;
  stickyFirstColumn?: boolean;
}

export default function ResponsiveTable({
  children,
  className = '',
  testId,
  stickyFirstColumn = false,
}: ResponsiveTableProps) {
  return (
    <div
      className={`min-w-0 overflow-x-auto ${stickyFirstColumn ? 'responsive-table-sticky-first' : ''} ${className}`.trim()}
      data-testid={testId}
    >
      {children}
    </div>
  );
}
