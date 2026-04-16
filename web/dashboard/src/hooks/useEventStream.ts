import { useEffect, useRef, useCallback, useState } from 'react';

interface ServerEvent {
  type: 'workloadChanged' | 'eventEmitted' | 'healthUpdate';
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

    const url = `${apiUrl}/api/events/stream`;
    const es = new EventSource(url);
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
