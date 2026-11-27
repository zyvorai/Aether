// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { apiFetchWithError, apiText } from '../../utils/api';
import { formatTimestamp } from '../../utils/formatters';
import { levelTone } from '../../utils/operator';
import type { Change } from '../../types/operator';
import Badge from '../Badge';
import PageLoading from '../PageLoading';
import PageLoadError from '../PageLoadError';

/** Load `path`; a slower, older response never replaces a newer one. */
export function useApi<T>(path: string | null, refreshKey?: number) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(path !== null);
  const latest = useRef(0);
  const shownPath = useRef(path);

  const reload = useCallback(async () => {
    const ticket = ++latest.current;
    if (shownPath.current !== path) {
      shownPath.current = path;
      setData(null);
    }
    if (path === null) {
      setError(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    const result = await apiFetchWithError<T>(path);
    if (ticket !== latest.current) return;
    setData(result.data);
    setError(result.error);
    setLoading(false);
  }, [path]);

  useEffect(() => {
    void reload();
  }, [reload, refreshKey]);

  return { data, error, loading, reload };
}

export function Loadable({
  loading,
  error,
  hasData,
  title,
  onRetry,
  children,
}: {
  loading: boolean;
  error: string | null;
  hasData: boolean;
  title: string;
  onRetry: () => void;
  children: ReactNode;
}) {
  if (!hasData && error) return <PageLoadError title={title} description={error} onRetry={onRetry} />;
  if (!hasData && loading) return <PageLoading rows={4} />;
  return <>{children}</>;
}

export function Section({ title, actions, children }: { title: string; actions?: ReactNode; children: ReactNode }) {
  return (
    <section className="glass p-5" aria-label={title}>
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <h2 className="text-lg font-semibold text-foreground">{title}</h2>
        {actions}
      </div>
      {children}
    </section>
  );
}

export function ErrorText({ children }: { children: ReactNode }) {
  return children ? (
    <p role="alert" className="mt-2 text-sm text-danger">
      {children}
    </p>
  ) : null;
}

export function ChangeList({ changes, empty }: { changes: Change[]; empty: string }) {
  if (changes.length === 0) return <p className="text-sm text-muted">{empty}</p>;
  return (
    <ol className="divide-y divide-[var(--border)]">
      {changes.map((c, i) => (
        <li key={`${c.at}-${c.title}-${i}`} className="flex flex-col gap-1 py-2.5 sm:flex-row sm:gap-4">
          <time className="shrink-0 font-mono text-xs text-muted" dateTime={c.at}>
            {formatTimestamp(c.at)}
          </time>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2 text-sm">
              {c.level !== 'info' ? <Badge text={c.level} variant={levelTone(c.level)} /> : null}
              <span className="font-medium text-foreground">{c.title}</span>
              {c.workload ? <span className="font-mono text-xs text-muted">{c.workload}</span> : null}
            </div>
            <p className="break-words text-sm text-muted">
              {c.detail}
              {c.actor ? ` · by ${c.actor}` : ''}
            </p>
            {c.fields.length > 0 ? (
              <p className="break-all font-mono text-xs text-subtle">{c.fields.join(', ')}</p>
            ) : null}
          </div>
        </li>
      ))}
    </ol>
  );
}

/** Fetch a text endpoint with the session's credentials and save it as a file. */
export async function downloadText(path: string, filename: string, mime: string): Promise<boolean> {
  const body = await apiText(path);
  // Errors come back as the JSON envelope (or a fetch failure), not the document.
  if (body === 'Failed to fetch' || body.startsWith('{"success":false')) return false;
  const url = URL.createObjectURL(new Blob([body], { type: mime }));
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
  return true;
}
