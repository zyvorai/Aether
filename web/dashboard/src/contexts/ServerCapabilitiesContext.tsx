import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react';
import { apiFetch, primeServerSafety, type ServerSafety } from '../utils/api';
import type { PlatformInfo, SystemReadyStatus } from '../types/api';

export interface ServerCapabilities {
  version: string;
  safety: ServerSafety;
  platform: PlatformInfo;
}

interface ServerCapabilitiesContextValue {
  capabilities: ServerCapabilities | null;
  ready: SystemReadyStatus | null;
  loading: boolean;
  refreshPlatform: () => Promise<void>;
}

const ServerCapabilitiesContext = createContext<ServerCapabilitiesContextValue>({
  capabilities: null,
  ready: null,
  loading: true,
  refreshPlatform: async () => {},
});

function parsePlatformPayload(data: Record<string, unknown>): PlatformInfo | null {
  const version = typeof data.version === 'string' ? data.version : '';
  const safetyRaw = data.safety as Partial<ServerSafety> | undefined;
  const workloadRaw = data.workload_state as Record<string, unknown> | undefined;
  if (!safetyRaw) return null;

  const safety: ServerSafety = {
    mutation_confirm_required: Boolean(safetyRaw.mutation_confirm_required),
    mutation_confirm_header: safetyRaw.mutation_confirm_header ?? 'X-Aether-Confirm',
    mutation_confirm_values: safetyRaw.mutation_confirm_values ?? ['1'],
  };

  const oidcRaw = (data.oidc ?? {}) as Record<string, unknown>;
  const backend =
    workloadRaw?.backend === 'postgresql' || data.persistence === 'postgresql'
      ? 'postgresql'
      : 'local-json';

  const opaRaw = (data.opa ?? {}) as Record<string, unknown>;

  return {
    version,
    persistence: backend,
    haMode: typeof data.ha_mode === 'string' ? data.ha_mode : 'single',
    haSharedCache: Boolean(data.ha_shared_cache),
    tls: Boolean(data.tls),
    opa: {
      configured: Boolean(opaRaw.configured),
      enforce: Boolean(opaRaw.enforce),
      check_path: typeof opaRaw.check_path === 'string' ? opaRaw.check_path : '/api/policy/opa',
    },
    workloadState: {
      backend,
      configured: Boolean(workloadRaw?.configured ?? backend === 'postgresql'),
      pollSecs: typeof workloadRaw?.poll_secs === 'number' ? workloadRaw.poll_secs : 2,
      env: typeof workloadRaw?.env === 'string' ? workloadRaw.env : 'AETHER_STATE_DATABASE_URL',
      pollEnv: typeof workloadRaw?.poll_env === 'string' ? workloadRaw.poll_env : 'AETHER_STATE_POLL_SECS',
    },
    oidc: {
      enabled: Boolean(oidcRaw.enabled),
      issuer: typeof oidcRaw.issuer === 'string' ? oidcRaw.issuer : null,
    },
    safety,
  };
}

export function ServerCapabilitiesProvider({ children }: { children: ReactNode }) {
  const [capabilities, setCapabilities] = useState<ServerCapabilities | null>(null);
  const [ready, setReady] = useState<SystemReadyStatus | null>(null);
  const [loading, setLoading] = useState(true);

  const refreshPlatform = useCallback(async () => {
    const server = await apiFetch<Record<string, unknown>>('/server');
    let systemReady: SystemReadyStatus | null = null;
    try {
      const res = await fetch('/api/system/ready', { credentials: 'include' });
      if (res.ok || res.status === 503) {
        systemReady = (await res.json()) as SystemReadyStatus;
      }
    } catch {
      systemReady = null;
    }

    if (server) {
      const platform = parsePlatformPayload(server);
      if (platform) {
        primeServerSafety(platform.safety);
        setCapabilities({
          version: platform.version,
          safety: platform.safety,
          platform,
        });
      } else {
        primeServerSafety(null);
      }
    } else {
      primeServerSafety(null);
    }

    setReady(systemReady);
    setLoading(false);
  }, []);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      await refreshPlatform();
      if (cancelled) return;
    })();
    const interval = setInterval(() => {
      void refreshPlatform();
    }, 12_000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [refreshPlatform]);

  return (
    <ServerCapabilitiesContext.Provider value={{ capabilities, ready, loading, refreshPlatform }}>
      {children}
    </ServerCapabilitiesContext.Provider>
  );
}

export function useServerCapabilities() {
  return useContext(ServerCapabilitiesContext);
}
