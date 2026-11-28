// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useEffect, useState } from 'react';
import { Link } from 'react-router';
import { Bell } from 'lucide-react';
import type { Event } from '../../../types/api';
import { apiFetchSettled } from '../../../utils/api';
import { formatRelativeTime } from '../../../utils/formatters';
import { viewToPath } from '../../../utils/dashboardRoutes';
import { pathWithQuery } from '../../../utils/urlState';
import { SeverityBadge } from '../../Badge';
import NavMenu from './NavMenu';
import styles from './GlobalNav.module.css';

const SHOWN = 6;

/** Unacknowledged events, from any page. */
export default function EventInbox({ refreshKey = 0 }: { refreshKey?: number }) {
  // null: not loaded or failed. Never shown as zero.
  const [open, setOpen] = useState<Event[] | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let cancelled = false;
    void apiFetchSettled<Event[]>('/events').then((result) => {
      if (cancelled) return;
      setFailed(!result.ok);
      setOpen(
        result.ok && Array.isArray(result.data)
          ? result.data.filter((e) => !e.acknowledged).sort((a, b) => b.timestamp.localeCompare(a.timestamp))
          : null,
      );
    });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  const count = open?.length ?? 0;

  return (
    <NavMenu
      kind="dialog"
      title="Notifications"
      label={open === null ? 'Notifications' : `Notifications, ${count} unacknowledged`}
      triggerClassName={`${styles.icon} ${styles.inboxBtn}`}
      trigger={
        <>
          <Bell size={17} aria-hidden />
          {count > 0 ? <span className={styles.inboxCount} aria-hidden>{count > 99 ? '99+' : count}</span> : null}
        </>
      }
    >
      {(close) => (
        <div className="w-[min(22rem,calc(100vw-1.5rem))] p-1.5">
          <p className="px-2 pb-1.5 pt-1 text-sm font-semibold text-foreground">Unacknowledged events</p>
          {open === null ? (
            <p className="px-2 pb-2 text-sm text-muted">{failed ? 'Events could not be loaded.' : 'Loading…'}</p>
          ) : count === 0 ? (
            <p className="px-2 pb-2 text-sm text-muted">No unacknowledged events.</p>
          ) : (
            <ul className="max-h-[min(24rem,60dvh)] overflow-y-auto">
              {open.slice(0, SHOWN).map((event, i) => (
                <li key={`${event.timestamp}-${i}`}>
                  <Link
                    to={pathWithQuery(viewToPath('events'), { workload: event.workload ?? undefined })}
                    onClick={close}
                    className="block rounded-lg px-2 py-2 hover:bg-hover"
                  >
                    <span className="flex items-center gap-2">
                      <SeverityBadge severity={event.severity} />
                      <span className="min-w-0 flex-1 truncate text-sm font-medium text-foreground">{event.title}</span>
                      <span className="shrink-0 text-[11px] text-subtle">{formatRelativeTime(event.timestamp)}</span>
                    </span>
                    <span className="mt-0.5 block truncate text-xs text-muted">{event.message}</span>
                  </Link>
                </li>
              ))}
            </ul>
          )}
          <Link to={viewToPath('events')} onClick={close} className="mt-1 block border-t border-border px-2 py-2 text-sm text-primary hover:underline">
            {count > SHOWN ? `All events (${count - SHOWN} more unacknowledged)` : 'All events'}
          </Link>
        </div>
      )}
    </NavMenu>
  );
}
