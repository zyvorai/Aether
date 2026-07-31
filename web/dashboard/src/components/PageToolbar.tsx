// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef, useState, type ReactNode } from 'react';
import { RefreshCw, Search } from 'lucide-react';

interface PageToolbarProps {
  search?: string;
  onSearchChange?: (value: string) => void;
  searchPlaceholder?: string;
  searchTestId?: string;
  onRefresh?: () => void;
  refreshTestId?: string;
  refreshing?: boolean;
  filters?: ReactNode;
  actions?: ReactNode;
}

export default function PageToolbar({
  search,
  onSearchChange,
  searchPlaceholder = 'Search…',
  searchTestId,
  onRefresh,
  refreshTestId = 'page-toolbar-refresh',
  refreshing = false,
  filters,
  actions,
}: PageToolbarProps) {
  // `search` is typically backed by the URL query string (useQueryParam), which round-trips
  // through react-router's async setSearchParams. Binding the input directly to that prop
  // means a re-render can land with a stale value mid-keystroke and stomp the DOM value,
  // silently dropping characters on fast typing or paste. Buffer locally instead, and only
  // resync from the prop when it changes for a reason other than our own last edit (e.g. a
  // "clear filters" action or browser back/forward).
  const [localSearch, setLocalSearch] = useState(search ?? '');
  const lastPropagated = useRef(search ?? '');

  useEffect(() => {
    if ((search ?? '') !== lastPropagated.current) {
      lastPropagated.current = search ?? '';
      setLocalSearch(search ?? '');
    }
  }, [search]);

  return (
    <div className="glass-toolbar">
      <div className="flex flex-1 flex-wrap items-center gap-2 min-w-0">
        {onSearchChange !== undefined && (
          <div className="relative flex-1 min-w-[12rem] max-w-md">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-aether/70" aria-hidden />
            <input
              type="search"
              data-testid={searchTestId}
              value={localSearch}
              onChange={(e) => {
                const value = e.target.value;
                lastPropagated.current = value;
                setLocalSearch(value);
                onSearchChange(value);
              }}
              placeholder={searchPlaceholder}
              className="glass-input pl-9"
            />
          </div>
        )}
        {filters}
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {onRefresh && (
          <button
            type="button"
            data-testid={refreshTestId}
            onClick={onRefresh}
            disabled={refreshing}
            className="interactive-lift inline-flex items-center gap-2 rounded-xl border border-aether/20 bg-aether/10 px-3 py-2.5 text-sm font-medium text-aether transition hover:bg-aether/15 disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-aether/40"
          >
            <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
            Refresh
          </button>
        )}
        {actions}
      </div>
    </div>
  );
}
