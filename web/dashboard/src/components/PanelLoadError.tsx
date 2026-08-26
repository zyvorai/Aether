// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { RefreshCw, WifiOff } from 'lucide-react';

interface PanelLoadErrorProps {
  title: string;
  description?: string;
  onRetry: () => void;
}

export default function PanelLoadError({
  title,
  description = 'Could not load this section.',
  onRetry,
}: PanelLoadErrorProps) {
  return (
    <div className="glass-panel-card flex flex-col items-center gap-3 rounded-xl border border-red-500/20 bg-red-500/5 p-6 text-center">
      <WifiOff className="h-8 w-8 text-red-400/80" aria-hidden />
      <div>
        <p className="font-medium text-ink">{title}</p>
        <p className="mt-1 text-sm text-ink-3">{description}</p>
      </div>
      <button type="button" onClick={onRetry} className="btn-secondary inline-flex items-center gap-2 text-sm">
        <RefreshCw className="h-4 w-4" />
        Retry
      </button>
    </div>
  );
}
