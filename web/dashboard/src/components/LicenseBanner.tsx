// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { ShieldAlert } from 'lucide-react';

const ERROR_STATES = new Set([
  'EXPIRED_BLOCKED',
  'OVER_LIMIT_BLOCKED',
  'INVALID_SIGNATURE',
  'MISSING',
  'WRONG_PRODUCT',
]);

interface LicenseBannerProps {
  message: string;
  state: string;
}

export default function LicenseBanner({ message, state }: LicenseBannerProps) {
  const isError = ERROR_STATES.has(state);
  const colorClass = isError
    ? 'border-red-500/30 bg-red-500/10 text-red-200'
    : 'border-amber-500/30 bg-amber-500/10 text-amber-200';
  const iconClass = isError ? 'text-red-400' : 'text-amber-400';

  return (
    <div role="alert" className={`border-b ${colorClass} px-4 py-2.5 text-sm`}>
      <div className="dash-content flex items-center gap-2">
        <ShieldAlert className={`h-4 w-4 shrink-0 ${iconClass}`} aria-hidden />
        <span>{message}</span>
      </div>
    </div>
  );
}
