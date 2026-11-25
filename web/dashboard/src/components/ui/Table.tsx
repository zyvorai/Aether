// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import type { ReactNode } from 'react';
import { cn } from '../../lib/cn';

export function Table({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn('glass overflow-x-auto', className)}><table className="w-full text-body-sm">{children}</table></div>;
}
export function TableHead({ children }: { children: ReactNode }) {
  return <thead className="border-b border-border text-muted text-left">{children}</thead>;
}
export function TableBody({ children }: { children: ReactNode }) {
  return <tbody>{children}</tbody>;
}
export function TableRow({ children, className, onClick }: { children: ReactNode; className?: string; onClick?: () => void }) {
  return <tr className={cn('border-t border-border first:border-t-0 hover:bg-hover', className)} onClick={onClick}>{children}</tr>;
}
export function TableHeaderCell({ children, className }: { children: ReactNode; className?: string }) {
  return <th className={cn('px-4 py-3 text-xs font-semibold text-muted', className)}>{children}</th>;
}
export function TableCell({ children, className }: { children: ReactNode; className?: string }) {
  return <td className={cn('px-4 py-3', className)}>{children}</td>;
}
