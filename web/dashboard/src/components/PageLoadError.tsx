// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useState } from 'react';
import { Check, Copy, RefreshCw, WifiOff } from 'lucide-react';
import { copyToClipboard } from '../utils/clipboard';

interface PageLoadErrorProps {
  title: string;
  description?: string;
  /** The failing request and the server's answer, when known. */
  detail?: string;
  onRetry: () => void | Promise<unknown>;
}

export default function PageLoadError({
  title,
  description = 'Could not load data from the API. Check that aether serve is running.',
  detail,
  onRetry,
}: PageLoadErrorProps) {
  const [retrying, setRetrying] = useState(false);
  const [copied, setCopied] = useState(false);

  const retry = async () => {
    setRetrying(true);
    try {
      await onRetry();
    } finally {
      setRetrying(false);
    }
  };

  return (
    <div className="glass-empty-state state-panel" role="alert">
      <WifiOff size={36} className="text-subtle" aria-hidden />
      <h3 className="mt-3 text-lg font-semibold text-foreground">{title}</h3>
      <p className="mt-1.5 max-w-md text-sm leading-relaxed text-muted">{description}</p>
      {detail ? <code className="state-detail">{detail}</code> : null}
      <div className="mt-5 flex flex-wrap justify-center gap-2">
        <button type="button" onClick={() => void retry()} disabled={retrying} className="btn-primary inline-flex items-center gap-2">
          <RefreshCw className={`h-4 w-4 ${retrying ? 'animate-spin' : ''}`} aria-hidden />
          Retry
        </button>
        {detail ? (
          <button
            type="button"
            className="btn-secondary inline-flex items-center gap-2"
            onClick={() => {
              void copyToClipboard(`${title}\n${detail}\n${window.location.href}`).then((ok) => setCopied(ok));
            }}
          >
            {copied ? <Check className="h-4 w-4" aria-hidden /> : <Copy className="h-4 w-4" aria-hidden />}
            {copied ? 'Copied' : 'Copy details'}
          </button>
        ) : null}
      </div>
    </div>
  );
}
