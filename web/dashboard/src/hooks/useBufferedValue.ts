// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

import { useEffect, useRef, useState } from 'react';

/**
 * Buffers a controlled input's value locally instead of binding it directly to `value`/`onChange`.
 *
 * `value` is typically backed by the URL query string (useQueryParam/useSearchParams), which
 * round-trips through react-router's async setSearchParams. Binding an input directly to that
 * prop means a re-render can land mid-keystroke with a stale value and stomp the DOM value,
 * silently dropping characters on fast typing or paste. This buffers locally and only resyncs
 * from `value` when it changes for a reason other than our own last edit (e.g. a "clear filters"
 * button, browser back/forward, or a deep link).
 */
export function useBufferedValue(value: string, onChange: (next: string) => void): [string, (next: string) => void] {
  const [local, setLocal] = useState(value);
  const lastPropagated = useRef(value);

  useEffect(() => {
    if (value !== lastPropagated.current) {
      lastPropagated.current = value;
      setLocal(value);
    }
  }, [value]);

  const setBuffered = (next: string) => {
    lastPropagated.current = next;
    setLocal(next);
    onChange(next);
  };

  return [local, setBuffered];
}
