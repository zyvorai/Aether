// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { Eye } from 'lucide-react';
import { useAuth } from '../contexts/AuthContext';

export default function ViewerBanner() {
  const { isViewer } = useAuth();

  if (!isViewer) return null;

  return (
    <div role="status" className="glass-context-banner rounded-none border-x-0 border-t-0">
      <div className="dash-content flex items-center gap-2 text-sm text-ink-2">
        <Eye className="h-4 w-4 shrink-0 text-brand" aria-hidden />
        <span>Read-only session — deploy and mutation actions are disabled for the Viewer role.</span>
      </div>
    </div>
  );
}
