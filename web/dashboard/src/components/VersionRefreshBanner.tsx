// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useState } from 'react';
import { RefreshCw, X } from 'lucide-react';
import { apiFetch } from '../utils/api';

interface VersionPayload {
  api_version?: string;
  embedded_ui_build?: string;
}

const DISMISS_KEY = 'aether_dashboard_build_dismissed';

function buildsMatch(local: string, remote: string): boolean {
  if (local === remote) return true;
  // Local dev + API without AETHER_DASHBOARD_BUILD both stamp "dev".
  if (local === 'dev' || remote === 'dev') return import.meta.env.DEV;
  return false;
}

export default function VersionRefreshBanner() {
  const [remoteBuild, setRemoteBuild] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(() => {
    try {
      return sessionStorage.getItem(DISMISS_KEY) === '1';
    } catch {
      return false;
    }
  });
  const localBuild = __AETHER_DASHBOARD_BUILD__;

  useEffect(() => {
    const load = async () => {
      const data = await apiFetch<VersionPayload>('/dashboard/version');
      if (data?.embedded_ui_build) {
        setRemoteBuild(data.embedded_ui_build);
      }
    };
    void load();
    const id = window.setInterval(() => void load(), 120_000);
    return () => window.clearInterval(id);
  }, []);

  if (dismissed || !remoteBuild || buildsMatch(localBuild, remoteBuild)) {
    return null;
  }

  const dismiss = () => {
    setDismissed(true);
    try {
      sessionStorage.setItem(DISMISS_KEY, '1');
    } catch {
      /* ignore */
    }
  };

  return (
    <div
      role="status"
      className="relative border-b border-warning/50 bg-warning/40 px-4 py-2 pr-10 text-center text-sm text-warning"
    >
      <RefreshCw className="inline-block w-4 h-4 mr-2 -mt-0.5" aria-hidden />
      A new dashboard build is available on the server.{' '}
      <button
        type="button"
        onClick={() => window.location.reload()}
        className="underline font-medium hover:text-foreground"
      >
        Reload to update
      </button>
      <span className="ml-2 hidden sm:inline text-xs text-warning/80 font-mono">
        server {remoteBuild.slice(0, 19)} · local {localBuild.slice(0, 19)}
      </span>
      <button
        type="button"
        onClick={dismiss}
        className="absolute right-3 top-1/2 -translate-y-1/2 rounded p-1 text-warning/80 hover:text-foreground"
        aria-label="Dismiss update notice"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
