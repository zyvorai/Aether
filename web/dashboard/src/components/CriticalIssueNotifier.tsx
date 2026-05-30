// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef } from 'react';
import { apiFetch } from '../utils/api';
import type { CriticalNotificationsReport } from '../types/api';

const SEEN_KEY = 'aether-critical-notifications-seen';

function loadSeen(): Set<string> {
  try {
    const raw = sessionStorage.getItem(SEEN_KEY);
    if (!raw) return new Set();
    return new Set(JSON.parse(raw) as string[]);
  } catch {
    return new Set();
  }
}

function saveSeen(seen: Set<string>) {
  try {
    sessionStorage.setItem(SEEN_KEY, JSON.stringify([...seen].slice(-50)));
  } catch {
    /* ignore */
  }
}

interface CriticalIssueNotifierProps {
  refreshKey?: number;
  enabled?: boolean;
}

export default function CriticalIssueNotifier({ refreshKey = 0, enabled = true }: CriticalIssueNotifierProps) {
  const seenRef = useRef(loadSeen());

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;

    apiFetch<CriticalNotificationsReport>('/command-center/notifications').then((report) => {
      if (cancelled || !report?.notifications.length) return;

      const fresh = report.notifications.filter((n) => !seenRef.current.has(n.id));
      if (fresh.length === 0) return;

      for (const note of fresh.slice(0, 3)) {
        window.dispatchEvent(
          new CustomEvent('aether-toast', {
            detail: {
              message: `${note.title}: ${note.detail}`,
              type: note.severity === 'critical' ? 'error' : 'info',
            },
          }),
        );
        seenRef.current.add(note.id);
      }
      saveSeen(seenRef.current);

      const top = fresh[0];
      window.dispatchEvent(
        new CustomEvent('aether-tray-critical', {
          detail: { title: top.title, detail: top.detail, count: fresh.length },
        }),
      );

      const tauri = (window as Window & {
        __TAURI__?: { core?: { invoke?: (cmd: string, args: object) => Promise<unknown> } };
      }).__TAURI__;
      void tauri?.core?.invoke?.('tray_critical_alert', {
        title: top.title,
        detail: top.detail,
        count: fresh.length,
      }).catch(() => {});
    });

    return () => {
      cancelled = true;
    };
  }, [refreshKey, enabled]);

  return null;
}
