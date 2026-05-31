// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { RefreshCw, WifiOff } from 'lucide-react';
import EmptyState from './EmptyState';

interface PageLoadErrorProps {
  title: string;
  description?: string;
  onRetry: () => void;
}

export default function PageLoadError({
  title,
  description = 'Could not load data from the API. Check that aether serve is running.',
  onRetry,
}: PageLoadErrorProps) {
  return (
    <EmptyState
      icon={<WifiOff size={48} />}
      title={title}
      description={description}
      action={
        <button
          type="button"
          onClick={onRetry}
          className="btn-primary inline-flex items-center gap-2"
        >
          <RefreshCw className="h-4 w-4" />
          Retry
        </button>
      }
    />
  );
}
