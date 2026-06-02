// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { AlertCircle } from 'lucide-react';

interface InlineActionErrorProps {
  message: string;
  id?: string;
}

export default function InlineActionError({ message, id }: InlineActionErrorProps) {
  return (
    <div
      id={id}
      role="alert"
      className="glass-panel-card flex items-start gap-2 rounded-xl border border-red-500/30 bg-red-500/5 px-4 py-3 text-sm text-red-300"
    >
      <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden />
      <span>{message}</span>
    </div>
  );
}
