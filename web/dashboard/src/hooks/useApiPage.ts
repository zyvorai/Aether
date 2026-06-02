// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useCallback, useEffect, useRef, useState } from 'react';

export interface UseApiPageResult<T> {
  data: T | null;
  loading: boolean;
  error: boolean;
  refreshing: boolean;
  reload: () => Promise<void>;
}

export function useApiPage<T>(
  fetcher: () => Promise<{ ok: true; data: T } | { ok: false; error?: string }>,
  deps: readonly unknown[] = [],
): UseApiPageResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const hasDataRef = useRef(false);

  const reload = useCallback(async () => {
    const isBackground = hasDataRef.current;
    if (isBackground) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(false);

    const result = await fetcher();
    if (result.ok) {
      setData(result.data);
      hasDataRef.current = true;
    } else {
      if (!hasDataRef.current) {
        setData(null);
        setError(true);
      }
    }

    setLoading(false);
    setRefreshing(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- caller controls invalidation via deps
  }, deps);

  useEffect(() => {
    hasDataRef.current = false;
    setData(null);
    void reload();
  }, [reload]);

  return { data, loading, error, refreshing, reload };
}

export type MultiFetchResults<T extends readonly unknown[]> = {
  [K in keyof T]: T[K];
};

export interface UseApiPageMultiResult<T extends readonly unknown[]> {
  data: MultiFetchResults<T> | null;
  loading: boolean;
  error: boolean;
  refreshing: boolean;
  reload: () => Promise<void>;
}

export function useApiPageMulti<T extends readonly unknown[]>(
  fetcher: () => Promise<
    { ok: true; data: MultiFetchResults<T> } | { ok: false; error?: string }
  >,
  deps: readonly unknown[] = [],
): UseApiPageMultiResult<T> {
  const [data, setData] = useState<MultiFetchResults<T> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const hasDataRef = useRef(false);

  const reload = useCallback(async () => {
    const isBackground = hasDataRef.current;
    if (isBackground) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(false);

    const result = await fetcher();
    if (result.ok) {
      setData(result.data);
      hasDataRef.current = true;
    } else {
      if (!hasDataRef.current) {
        setData(null);
        setError(true);
      }
    }

    setLoading(false);
    setRefreshing(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  useEffect(() => {
    hasDataRef.current = false;
    setData(null);
    void reload();
  }, [reload]);

  return { data, loading, error, refreshing, reload };
}
