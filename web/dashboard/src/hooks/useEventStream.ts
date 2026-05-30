// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

import { useEffect, useRef, useCallback, useState } from 'react';
import { apiEventStreamUrl } from '../utils/api';

interface ServerEvent {
  type: 'workloadChanged' | 'eventEmitted' | 'healthUpdate' | 'migrationProgress';
  workload?: string;
  phase?: string;
  percent?: number;
  eta_secs?: number;
  message?: string;
  [key: string]: unknown;
}

export function useEventStream(
  apiUrl: string,
  onEvent: (event: ServerEvent) => void,
  enabled: boolean = true
) {
  const [connected, setConnected] = useState(false);
  const eventSourceRef = useRef<EventSource | null>(null);
  const onEventRef = useRef(onEvent);
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!enabled) {
      setConnected(false);
      return;
    }

    const streamPath = apiEventStreamUrl();
    const url = apiUrl ? `${apiUrl}${streamPath}` : streamPath;
    const es = new EventSource(url, { withCredentials: true });
    eventSourceRef.current = es;

    es.onopen = () => setConnected(true);

    es.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data) as ServerEvent;
        onEventRef.current(data);
      } catch {
        // ignore parse errors
      }
    };

    es.onerror = () => {
      setConnected(false);
      // EventSource auto-reconnects
    };

    return () => {
      es.close();
      eventSourceRef.current = null;
      setConnected(false);
    };
  }, [apiUrl, enabled]);

  const close = useCallback(() => {
    eventSourceRef.current?.close();
    eventSourceRef.current = null;
    setConnected(false);
  }, []);

  return { close, connected };
}
