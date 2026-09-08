// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
      className="glass flex items-start gap-2 rounded-xl border border-danger/30 bg-danger/5 px-4 py-3 text-sm text-danger"
    >
      <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden />
      <span>{message}</span>
    </div>
  );
}
