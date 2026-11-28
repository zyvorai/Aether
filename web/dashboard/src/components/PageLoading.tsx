// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';

interface PageLoadingProps {
  rows?: number;
  variant?: 'skeleton' | 'spinner' | 'table';
  label?: string;
  className?: string;
}

const SLOW_AFTER_MS = 6000;

/** Says so when a load takes long, instead of shimmering silently. */
function useSlow(): boolean {
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    const timer = window.setTimeout(() => setSlow(true), SLOW_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, []);
  return slow;
}

export default function PageLoading({ rows = 4, variant = 'skeleton', label, className }: PageLoadingProps) {
  const slow = useSlow();

  if (variant === 'spinner' || label) {
    return (
      <div className={`flex flex-col items-center justify-center gap-3 ${className ?? 'py-16'}`} role="status">
        <div className="spinner animate-spin" aria-hidden />
        <p className={label || slow ? 'text-sm text-subtle' : 'sr-only'}>
          {label ?? (slow ? 'Still waiting for the server…' : 'Loading…')}
        </p>
      </div>
    );
  }

  return (
    <div className={`glass state-skeleton ${className ?? ''}`} role="status" aria-busy="true">
      <span className="sr-only">Loading…</span>
      {Array.from({ length: rows }).map((_, i) =>
        variant === 'table' ? (
          <div key={i} className="flex items-center gap-4 py-1.5" aria-hidden>
            <div className="skeleton h-4 flex-[2] rounded-md" />
            <div className="skeleton h-4 flex-1 rounded-md" />
            <div className="skeleton h-4 flex-[2] rounded-md max-sm:hidden" />
            <div className="skeleton h-4 w-20 rounded-md" />
          </div>
        ) : (
          <div key={i} className="skeleton h-16 rounded-xl" aria-hidden />
        ),
      )}
      {slow ? <p className="pt-2 text-center text-sm text-muted">Still waiting for the server…</p> : null}
    </div>
  );
}
