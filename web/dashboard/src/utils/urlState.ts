// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useMemo } from 'react';
import { useSearchParams } from 'react-router';

/** Read/write a single query param (empty string removes key). */
export function useQueryParam(key: string, defaultValue = ''): [string, (value: string) => void] {
  const [params, setParams] = useSearchParams();

  const value = params.get(key) ?? defaultValue;

  const setValue = useCallback(
    (next: string) => {
      setParams(
        (prev) => {
          const copy = new URLSearchParams(prev);
          if (!next || next === defaultValue) {
            copy.delete(key);
          } else {
            copy.set(key, next);
          }
          return copy;
        },
        { replace: true }
      );
    },
    [key, defaultValue, setParams]
  );

  return [value, setValue];
}

/**
 * Search pages: `?workload=` deep links alias `?q=` toolbar filter.
 * Setting the filter updates both params so bookmarks and palette links stay in sync.
 */
export function useWorkloadOrSearchFilter(): [string, (value: string) => void] {
  const [q, setQ] = useQueryParam('q');
  const [workload, setWorkload] = useQueryParam('workload');
  const effective = workload.trim() || q.trim();
  const setEffective = useCallback(
    (next: string) => {
      setQ(next);
      setWorkload(next);
    },
    [setQ, setWorkload],
  );
  return [effective, setEffective];
}

/** Append query string to a dashboard path (e.g. deep links from Overview). */
export function pathWithQuery(basePath: string, params: Record<string, string | undefined>): string {
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(params)) {
    if (v) qs.set(k, v);
  }
  const s = qs.toString();
  return s ? `${basePath}?${s}` : basePath;
}

export function useQueryParams(keys: string[]): Record<string, string> {
  const [params] = useSearchParams();
  return useMemo(() => {
    const out: Record<string, string> = {};
    for (const key of keys) {
      const v = params.get(key);
      if (v) out[key] = v;
    }
    return out;
  }, [params, keys]);
}
