import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { apiFetch, primeServerSafety, type ServerSafety } from '../utils/api';

export interface ServerCapabilities {
  version: string;
  safety: ServerSafety;
}

interface ServerCapabilitiesContextValue {
  capabilities: ServerCapabilities | null;
  loading: boolean;
}

const ServerCapabilitiesContext = createContext<ServerCapabilitiesContextValue>({
  capabilities: null,
  loading: true,
});

export function ServerCapabilitiesProvider({ children }: { children: ReactNode }) {
  const [capabilities, setCapabilities] = useState<ServerCapabilities | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const data = await apiFetch<{
        version?: string;
        safety?: Partial<ServerSafety>;
      }>('/server');
      if (cancelled) return;
      if (data?.safety) {
        const safety: ServerSafety = {
          mutation_confirm_required: Boolean(data.safety.mutation_confirm_required),
          mutation_confirm_header: data.safety.mutation_confirm_header ?? 'X-Aether-Confirm',
          mutation_confirm_values: data.safety.mutation_confirm_values ?? ['1'],
        };
        primeServerSafety(safety);
        setCapabilities({
          version: data.version ?? '',
          safety,
        });
      } else {
        primeServerSafety(null);
      }
      setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <ServerCapabilitiesContext.Provider value={{ capabilities, loading }}>
      {children}
    </ServerCapabilitiesContext.Provider>
  );
}

export function useServerCapabilities() {
  return useContext(ServerCapabilitiesContext);
}
