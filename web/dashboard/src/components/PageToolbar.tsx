// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import type { ReactNode } from 'react';
import { RefreshCw, Search } from 'lucide-react';
import { useBufferedValue } from '../hooks/useBufferedValue';
import { Button } from './ui/Button';

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
  const [localSearch, setLocalSearch] = useBufferedValue(search ?? '', onSearchChange ?? (() => {}));

  return (
    <div className="glass-toolbar">
      <div className="flex flex-1 flex-wrap items-center gap-2 min-w-0">
        {onSearchChange !== undefined && (
          <div className="relative flex-1 min-w-[12rem] max-w-md">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-primary/70" aria-hidden />
            <input
              type="search"
              data-testid={searchTestId}
              value={localSearch}
              onChange={(e) => setLocalSearch(e.target.value)}
              placeholder={searchPlaceholder}
              className="glass-input pl-9"
            />
          </div>
        )}
        {filters}
      </div>
      <div className="flex items-center gap-2 shrink-0">
        {onRefresh && (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            data-testid={refreshTestId}
            onClick={onRefresh}
            disabled={refreshing}
            className="gap-2"
          >
            <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
            Refresh
          </Button>
        )}
        {actions}
      </div>
    </div>
  );
}
